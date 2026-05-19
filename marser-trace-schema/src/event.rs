//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use serde::{Deserialize, Serialize};

use crate::rule::{RuleIdentity, TraceLocation};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TraceEventKind {
    MatchFail,
    MatchHardError,
    ParserEnter,
    ParserExit,
    /// Reserved for forward compatibility with traces produced by newer marser versions.
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeTraceKind {
    TraceMarker,
    ChoiceArm,
    CaptureBoundary,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeTraceStatus {
    Enter,
    Success,
    Fail,
    Backtrack,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraceMarkerPhase {
    #[default]
    None,
    Start,
    End,
}

/// How the inner parser/matcher finished for an explicit `.trace()` **End** event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplicitMarkerEndOutcome {
    /// `Some(_)` / matcher matched.
    Success,
    /// `None` / matcher did not match (no hard error).
    SoftFail,
    /// Hard error from the inner parse/match.
    HardError,
}

/// Compact error context attached to a `.trace()` **End** when the inner parse/match failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceMarkerFailureSnapshot {
    pub span_start: usize,
    pub span_end: usize,
    pub expected: Vec<String>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub trace_marker_id: Option<u64>,
    #[serde(default)]
    pub marker_phase: TraceMarkerPhase,
    #[serde(default)]
    pub is_explicit_trace_marker: bool,
    pub runtime_kind: Option<TraceEventKind>,
    pub rule: Option<RuleIdentity>,
    #[serde(default)]
    pub error_sink_len: usize,
    #[serde(default)]
    pub error_stack_len: usize,
    #[serde(default)]
    pub marker_failure: Option<TraceMarkerFailureSnapshot>,
}
