use std::collections::HashMap;

use crate::context::ParserContext;
use crate::trace::{
    ExplicitMarkerEndOutcome, NodeTrace, NodeTraceKind, NodeTraceStatus, RuleIdentity,
    RuleSourceMetadata, TraceEventKind, TraceLocation, TraceMarkerFailureSnapshot,
    TraceMarkerPhase, TraceSession,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RuleMetadataKey {
    file: &'static str,
    line: u32,
    column: u32,
    rule_name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TraceState {
    pub(crate) session: TraceSession,
    next_id: u64,
    next_marker_id: u64,
    next_rule_id: u64,
    rule_ids: HashMap<RuleMetadataKey, u64>,
}

impl<'src> ParserContext<'src> {
    #[inline]
    pub fn attach_trace_session(&mut self, session: TraceSession) {
        self.trace = Some(TraceState {
            session,
            next_id: 0,
            next_marker_id: 0,
            next_rule_id: 0,
            rule_ids: HashMap::new(),
        });
    }

    #[inline]
    pub fn take_trace_session(&mut self) -> Option<TraceSession> {
        self.trace.take().map(|t| t.session)
    }

    #[inline]
    pub fn next_trace_marker_id(&mut self) -> u64 {
        let trace = self.trace.get_or_insert_with(|| TraceState {
            session: TraceSession::new(),
            ..Default::default()
        });
        let id = trace.next_marker_id;
        trace.next_marker_id = trace.next_marker_id.saturating_add(1);
        id
    }

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
        debug_assert!(
            !matches!(phase, TraceMarkerPhase::None),
            "explicit trace marker events should not use phase None"
        );

        let (runtime_kind, status) = match phase {
            TraceMarkerPhase::Start => (TraceEventKind::ParserEnter, NodeTraceStatus::Enter),
            TraceMarkerPhase::End => {
                match end_outcome.expect("explicit trace End requires outcome") {
                    ExplicitMarkerEndOutcome::Success => {
                        (TraceEventKind::ParserExit, NodeTraceStatus::Success)
                    }
                    ExplicitMarkerEndOutcome::SoftFail => {
                        (TraceEventKind::MatchFail, NodeTraceStatus::Fail)
                    }
                    ExplicitMarkerEndOutcome::HardError => {
                        (TraceEventKind::MatchHardError, NodeTraceStatus::Fail)
                    }
                }
            }
            TraceMarkerPhase::None => unreachable!("explicit marker phase None is invalid"),
        };

        self.record_explicit_marker(
            marker_id,
            phase,
            runtime_kind,
            status,
            pos,
            label,
            usage_metadata,
            definition_metadata,
            marker_failure,
        );
    }

    fn record_explicit_marker(
        &mut self,
        marker_id: u64,
        marker_phase: TraceMarkerPhase,
        runtime_kind: TraceEventKind,
        status: NodeTraceStatus,
        pos: usize,
        label: Option<String>,
        usage_metadata: RuleSourceMetadata,
        definition_metadata: Option<RuleSourceMetadata>,
        marker_failure: Option<TraceMarkerFailureSnapshot>,
    ) {
        let Some(trace) = self.trace.as_mut() else {
            return;
        };

        let node_id = trace.next_id;
        trace.next_id = trace.next_id.saturating_add(1);

        let rule = Some(resolve_rule_identity(trace, usage_metadata, label.as_ref()));
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
        let marker_failure = if matches!(marker_phase, TraceMarkerPhase::End) {
            marker_failure
        } else {
            None
        };

        trace.session.record(NodeTrace {
            node_id,
            parent_node_id: None,
            usage_loc: usage_loc.clone(),
            definition_loc: definition_loc.or(usage_loc),
            kind: NodeTraceKind::Runtime,
            status,
            label,
            input_start: pos,
            input_end: pos,
            is_step_marker: true,
            trace_marker_id: Some(marker_id),
            marker_phase,
            is_explicit_trace_marker: true,
            runtime_kind: Some(runtime_kind),
            rule,
            error_sink_len: self.error_sink.len(),
            error_stack_len: self.error_stack.len(),
            marker_failure,
        });
    }
}

fn resolve_rule_identity(
    trace: &mut TraceState,
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
    let rule_id = if let Some(id) = trace.rule_ids.get(&key) {
        *id
    } else {
        let id = trace.next_rule_id;
        trace.next_rule_id = trace.next_rule_id.saturating_add(1);
        trace.rule_ids.insert(key, id);
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
