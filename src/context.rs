use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::error::ParserError;
#[cfg(feature = "parser-trace")]
use crate::trace::{
    ExplicitMarkerEndOutcome, NodeTrace, NodeTraceKind, NodeTraceStatus, RuleIdentity,
    RuleSourceMetadata, TraceEventKind, TraceLocation, TraceMarkerFailureSnapshot,
    TraceMarkerPhase, TraceSession,
};

#[cfg(feature = "parser-trace")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RuleMetadataKey {
    file: &'static str,
    line: u32,
    column: u32,
    rule_name: Option<String>,
}

pub struct ParserContext {
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
    pub error_stack: Vec<ParserError>,
    #[cfg(feature = "parser-trace")]
    trace_session: Option<TraceSession>,
    #[cfg(feature = "parser-trace")]
    trace_next_id: u64,
    #[cfg(feature = "parser-trace")]
    trace_parent_stack: Vec<u64>,
    #[cfg(feature = "parser-trace")]
    trace_next_marker_id: u64,
    #[cfg(feature = "parser-trace")]
    trace_next_rule_id: u64,
    #[cfg(feature = "parser-trace")]
    trace_rule_ids: HashMap<RuleMetadataKey, u64>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            memo_table: HashMap::new(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
            error_stack: Vec::new(),
            #[cfg(feature = "parser-trace")]
            trace_session: None,
            #[cfg(feature = "parser-trace")]
            trace_next_id: 0,
            #[cfg(feature = "parser-trace")]
            trace_parent_stack: Vec::new(),
            #[cfg(feature = "parser-trace")]
            trace_next_marker_id: 0,
            #[cfg(feature = "parser-trace")]
            trace_next_rule_id: 0,
            #[cfg(feature = "parser-trace")]
            trace_rule_ids: HashMap::new(),
        }
    }

    pub fn get_errors(mut self) -> Vec<ParserError> {
        // return combined errors from error_sink and error_stack
        self.error_sink.extend(self.error_stack);
        self.error_sink
    }

    pub fn push_stack_error(&mut self, error: ParserError) {
        self.error_stack.push(error);
    }

    #[cfg(feature = "parser-trace")]
    pub fn attach_trace_session(&mut self, session: TraceSession) {
        self.trace_session = Some(session);
        self.trace_next_id = 0;
        self.trace_parent_stack.clear();
        self.trace_next_marker_id = 0;
        self.trace_next_rule_id = 0;
        self.trace_rule_ids.clear();
    }

    #[cfg(feature = "parser-trace")]
    pub fn take_trace_session(&mut self) -> Option<TraceSession> {
        self.trace_session.take()
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_enabled(&self) -> bool {
        self.trace_session.is_some()
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_enter(
        &mut self,
        kind: TraceEventKind,
        pos: usize,
        label: Option<String>,
        metadata: Option<RuleSourceMetadata>,
    ) {
        let pushes_parent = matches!(kind, TraceEventKind::CaptureEnter | TraceEventKind::ParserEnter);
        self.emit_trace(kind, pos, pos, label, metadata, None);
        if pushes_parent {
            self.trace_parent_stack
                .push(self.trace_next_id.saturating_sub(1));
        }
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_enter_with_definition(
        &mut self,
        kind: TraceEventKind,
        pos: usize,
        label: Option<String>,
        usage_metadata: Option<RuleSourceMetadata>,
        definition_metadata: Option<RuleSourceMetadata>,
    ) {
        let pushes_parent = matches!(kind, TraceEventKind::CaptureEnter | TraceEventKind::ParserEnter);
        self.emit_trace(kind, pos, pos, label, usage_metadata, definition_metadata);
        if pushes_parent {
            self.trace_parent_stack
                .push(self.trace_next_id.saturating_sub(1));
        }
    }

    #[cfg(feature = "parser-trace")]
    pub fn next_trace_marker_id(&mut self) -> u64 {
        let id = self.trace_next_marker_id;
        self.trace_next_marker_id = self.trace_next_marker_id.saturating_add(1);
        id
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_explicit_marker(
        &mut self,
        marker_id: u64,
        phase: TraceMarkerPhase,
        pos: usize,
        label: Option<String>,
        usage_metadata: RuleSourceMetadata,
        definition_metadata: Option<RuleSourceMetadata>,
        end_outcome: Option<ExplicitMarkerEndOutcome>,
        marker_failure: Option<TraceMarkerFailureSnapshot>,
    ) {
        debug_assert!(
            marker_failure.is_none() || matches!(phase, TraceMarkerPhase::End),
            "marker_failure only allowed on explicit trace End"
        );
        let kind = match phase {
            TraceMarkerPhase::Start => TraceEventKind::ParserEnter,
            TraceMarkerPhase::End => match end_outcome.expect("explicit trace End requires outcome") {
                ExplicitMarkerEndOutcome::Success => TraceEventKind::ParserExit,
                ExplicitMarkerEndOutcome::SoftFail => TraceEventKind::MatchFail,
                ExplicitMarkerEndOutcome::HardError => TraceEventKind::MatchHardError,
            },
            TraceMarkerPhase::None => TraceEventKind::ParserEnter,
        };
        self.emit_trace_internal(
            kind,
            pos,
            pos,
            label,
            Some(usage_metadata),
            definition_metadata,
            Some(marker_id),
            phase,
            true,
            true,
            marker_failure,
        );
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_leave(&mut self) {
        let _ = self.trace_parent_stack.pop();
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_event(
        &mut self,
        kind: TraceEventKind,
        start: usize,
        end: usize,
        label: Option<String>,
        metadata: Option<RuleSourceMetadata>,
    ) {
        self.emit_trace(kind, start, end, label, metadata, None);
    }

    #[cfg(feature = "parser-trace")]
    pub fn trace_event_with_definition(
        &mut self,
        kind: TraceEventKind,
        start: usize,
        end: usize,
        label: Option<String>,
        usage_metadata: Option<RuleSourceMetadata>,
        definition_metadata: Option<RuleSourceMetadata>,
    ) {
        self.emit_trace(
            kind,
            start,
            end,
            label,
            usage_metadata,
            definition_metadata,
        );
    }

    #[cfg(feature = "parser-trace")]
    fn emit_trace(
        &mut self,
        kind: TraceEventKind,
        start: usize,
        end: usize,
        label: Option<String>,
        usage_metadata: Option<RuleSourceMetadata>,
        definition_metadata: Option<RuleSourceMetadata>,
    ) {
        self.emit_trace_internal(
            kind,
            start,
            end,
            label,
            usage_metadata,
            definition_metadata,
            None,
            TraceMarkerPhase::None,
            false,
            false,
            None,
        );
    }

    #[cfg(feature = "parser-trace")]
    fn emit_trace_internal(
        &mut self,
        kind: TraceEventKind,
        start: usize,
        end: usize,
        label: Option<String>,
        usage_metadata: Option<RuleSourceMetadata>,
        definition_metadata: Option<RuleSourceMetadata>,
        marker_id: Option<u64>,
        marker_phase: TraceMarkerPhase,
        is_explicit_trace_marker: bool,
        is_step_marker_override: bool,
        marker_failure: Option<TraceMarkerFailureSnapshot>,
    ) {
        let node_id = self.trace_next_id;
        self.trace_next_id = self.trace_next_id.saturating_add(1);
        let rule = usage_metadata.map(|meta| self.resolve_rule_identity(meta, label.as_ref()));
        let usage_loc = rule.as_ref().map(|r| TraceLocation {
            file: r.rule_file.clone(),
            line: r.rule_line,
            column: r.rule_column,
        });
        let definition_loc = definition_metadata.map(|meta| TraceLocation {
            file: meta.file.to_string(),
            line: meta.line,
            column: meta.column,
        });
        let kind_mapped = match kind {
            TraceEventKind::ChoiceArmStart
            | TraceEventKind::ChoiceArmSuccess
            | TraceEventKind::ChoiceArmFail => NodeTraceKind::ChoiceArm,
            TraceEventKind::CaptureEnter | TraceEventKind::CaptureExit => NodeTraceKind::CaptureBoundary,
            _ => NodeTraceKind::Runtime,
        };
        let status = match kind {
            TraceEventKind::MatchSuccess
            | TraceEventKind::ChoiceArmSuccess
            | TraceEventKind::CommitSecondPassSuccess
            | TraceEventKind::RecoverSuccess
            | TraceEventKind::ParserExit
            | TraceEventKind::CaptureExit => NodeTraceStatus::Success,
            TraceEventKind::MatchFail
            | TraceEventKind::ChoiceArmFail
            | TraceEventKind::ChoiceAllFailed
            | TraceEventKind::CommitSecondPassFail
            | TraceEventKind::RecoverFail
            | TraceEventKind::MatchHardError => NodeTraceStatus::Fail,
            TraceEventKind::MatchBacktrack => NodeTraceStatus::Backtrack,
            _ => NodeTraceStatus::Enter,
        };
        let is_step_marker = is_step_marker_override;
        let marker_failure = if is_explicit_trace_marker && matches!(marker_phase, TraceMarkerPhase::End)
        {
            marker_failure
        } else {
            None
        };
        if let Some(session) = self.trace_session.as_mut() {
            session.record(NodeTrace {
                node_id,
                parent_node_id: self.trace_parent_stack.last().copied(),
                usage_loc: usage_loc.clone(),
                definition_loc: definition_loc.or(usage_loc),
                kind: kind_mapped,
                status,
                label,
                input_start: start,
                input_end: end,
                is_step_marker,
                trace_marker_id: marker_id,
                marker_phase,
                is_explicit_trace_marker,
                runtime_kind: Some(kind),
                rule,
                error_sink_len: self.error_sink.len(),
                error_stack_len: self.error_stack.len(),
                marker_failure,
            });
        }
    }

    #[cfg(feature = "parser-trace")]
    fn resolve_rule_identity(
        &mut self,
        metadata: RuleSourceMetadata,
        label: Option<&String>,
    ) -> RuleIdentity {
        let derived_rule_name = metadata
            .rule_name
            .map(str::to_string)
            .or_else(|| label.cloned());
        let key = RuleMetadataKey {
            file: metadata.file,
            line: metadata.line,
            column: metadata.column,
            rule_name: derived_rule_name.clone(),
        };
        let rule_id = if let Some(id) = self.trace_rule_ids.get(&key) {
            *id
        } else {
            let id = self.trace_next_rule_id;
            self.trace_next_rule_id = self.trace_next_rule_id.saturating_add(1);
            self.trace_rule_ids.insert(key.clone(), id);
            id
        };
        RuleIdentity {
            rule_id,
            rule_name: derived_rule_name,
            rule_file: metadata.file.to_string(),
            rule_line: metadata.line,
            rule_column: metadata.column,
        }
    }
}
