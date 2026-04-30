use std::io::{self, Write};

#[cfg(feature = "parser-trace")]
use serde::{Deserialize, Serialize};

pub mod debug_protocol;
pub mod load;
pub mod render_text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleSourceMetadata {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub rule_name: Option<&'static str>,
}

impl RuleSourceMetadata {
    #[cfg(feature = "parser-trace")]
    pub const fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self {
            file,
            line,
            column,
            rule_name: None,
        }
    }

    #[cfg(feature = "parser-trace")]
    pub const fn with_rule_name(self, rule_name: &'static str) -> Self {
        Self {
            rule_name: Some(rule_name),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub struct RuleIdentity {
    pub rule_id: u64,
    pub rule_name: Option<String>,
    pub rule_file: String,
    pub rule_line: u32,
    pub rule_column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub enum TraceEventKind {
    MatchEnter,
    MatchSuccess,
    MatchFail,
    MatchBacktrack,
    MatchHardError,
    ChoiceStart,
    ChoiceArmStart,
    ChoiceArmSuccess,
    ChoiceArmFail,
    ChoiceAllFailed,
    CommitPrefixMatched,
    CommitSecondPassStart,
    CommitSecondPassSuccess,
    CommitSecondPassFail,
    RecoverAttempt,
    RecoverSuccess,
    RecoverFail,
    CaptureEnter,
    CaptureExit,
    ParserEnter,
    ParserExit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub struct TraceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub enum NodeTraceKind {
    TraceMarker,
    ChoiceArm,
    CaptureBoundary,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub enum NodeTraceStatus {
    Enter,
    Success,
    Fail,
    Backtrack,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub enum TraceMarkerPhase {
    #[default]
    None,
    Start,
    End,
}

/// How the inner parser/matcher finished for an explicit `.trace()` **End** event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub enum ExplicitMarkerEndOutcome {
    /// `Some(_)` / matcher matched.
    Success,
    /// `None` / matcher did not match (no `FurthestFailError`).
    SoftFail,
    /// `FurthestFailError` from the inner parse/match.
    HardError,
}

/// Compact error context attached to a `.trace()` **End** when the inner parse/match failed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub struct TraceMarkerFailureSnapshot {
    pub span_start: usize,
    pub span_end: usize,
    pub expected: Vec<String>,
    /// Same style as [`crate::error::FurthestFailError`]'s `Display` (e.g. `expected 'x' at 0..1`).
    pub summary: String,
}

#[cfg(feature = "parser-trace")]
impl From<&crate::error::FurthestFailError> for TraceMarkerFailureSnapshot {
    fn from(e: &crate::error::FurthestFailError) -> Self {
        Self {
            span_start: e.span.0,
            span_end: e.span.1,
            expected: e.expected.clone(),
            summary: e.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parser-trace", derive(Serialize, Deserialize))]
pub struct NodeTrace {
    pub node_id: u64,
    pub parent_node_id: Option<u64>,
    pub usage_loc: Option<TraceLocation>,
    pub definition_loc: Option<TraceLocation>,
    pub kind: NodeTraceKind,
    pub status: NodeTraceStatus,
    pub label: Option<String>,
    pub input_start: usize,
    pub input_end: usize,
    pub is_step_marker: bool,
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub trace_marker_id: Option<u64>,
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub marker_phase: TraceMarkerPhase,
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub is_explicit_trace_marker: bool,
    pub runtime_kind: Option<TraceEventKind>,
    pub rule: Option<RuleIdentity>,
    /// Snapshot of `ParserContext.error_sink.len()` when this event was recorded.
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub error_sink_len: usize,
    /// Snapshot of `ParserContext.error_stack.len()` when this event was recorded.
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub error_stack_len: usize,
    #[cfg_attr(feature = "parser-trace", serde(default))]
    pub marker_failure: Option<TraceMarkerFailureSnapshot>,
}

#[derive(Clone, Debug, Default)]
pub struct TraceSession {
    #[cfg(feature = "parser-trace")]
    nodes: Vec<NodeTrace>,
    #[cfg(feature = "parser-trace")]
    dropped_events: usize,
    #[cfg(feature = "parser-trace")]
    max_events: Option<usize>,
}

impl TraceSession {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "parser-trace")]
    pub fn with_max_events(max_events: usize) -> Self {
        Self {
            nodes: Vec::new(),
            dropped_events: 0,
            max_events: Some(max_events),
        }
    }

    #[cfg(feature = "parser-trace")]
    pub fn record(&mut self, node: NodeTrace) {
        if let Some(max_events) = self.max_events
            && self.nodes.len() >= max_events
        {
            self.dropped_events = self.dropped_events.saturating_add(1);
            return;
        }
        self.nodes.push(node);
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn record(&mut self, _node: NodeTrace) {}

    #[cfg(feature = "parser-trace")]
    pub fn nodes(&self) -> &[NodeTrace] {
        &self.nodes
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn nodes(&self) -> &[NodeTrace] {
        &[]
    }

    pub fn events(&self) -> &[NodeTrace] {
        self.nodes()
    }

    #[cfg(feature = "parser-trace")]
    pub fn dropped_events(&self) -> usize {
        self.dropped_events
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn dropped_events(&self) -> usize {
        0
    }

    #[cfg(feature = "parser-trace")]
    pub fn write_json<W: Write>(&self, mut writer: W) -> io::Result<()> {
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "trace_version": 2,
                "nodes": self.nodes,
            }),
        )?;
        Ok(())
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn write_json<W: Write>(&self, _writer: W) -> io::Result<()> {
        Ok(())
    }

    #[cfg(feature = "parser-trace")]
    pub fn write_jsonl<W: Write>(&self, mut writer: W) -> io::Result<()> {
        for node in &self.nodes {
            serde_json::to_writer(&mut writer, node)?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn write_jsonl<W: Write>(&self, _writer: W) -> io::Result<()> {
        Ok(())
    }

    pub fn to_text_tree(&self) -> String {
        render_text::render_tree(self.nodes())
    }

    pub fn to_timeline(&self) -> String {
        render_text::render_timeline(self.nodes())
    }

    #[cfg(feature = "parser-trace")]
    pub fn from_events(nodes: Vec<NodeTrace>) -> Self {
        Self {
            nodes,
            dropped_events: 0,
            max_events: None,
        }
    }
}

