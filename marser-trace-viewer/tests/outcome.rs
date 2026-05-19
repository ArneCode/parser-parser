//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::collections::HashMap;

use marser_trace_schema::{
    NodeTrace, NodeTraceKind, NodeTraceStatus, TraceEventKind, TraceMarkerPhase,
};
use marser_trace_viewer::outcome::{MarkerOutcome, outcome_for_marker_span};

#[test]
fn outcome_success_when_end_success() {
    let (events, map) = paired_marker(true);
    assert_eq!(
        outcome_for_marker_span(&events, 0, &map),
        MarkerOutcome::Success
    );
}

#[test]
fn outcome_fail_when_end_fail() {
    let (events, map) = paired_marker(false);
    assert_eq!(
        outcome_for_marker_span(&events, 0, &map),
        MarkerOutcome::Fail
    );
}

#[test]
fn outcome_recovered_when_error_stack_increases_inside_span() {
    let start = NodeTrace {
        node_id: 0,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: NodeTraceStatus::Enter,
        label: Some("m".to_string()),
        input_start: 0,
        input_end: 0,
        is_step_marker: true,
        trace_marker_id: Some(1),
        marker_phase: TraceMarkerPhase::Start,
        is_explicit_trace_marker: true,
        runtime_kind: Some(TraceEventKind::ParserEnter),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 0,
        marker_failure: None,
    };
    let mid = NodeTrace {
        node_id: 1,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: NodeTraceStatus::Success,
        label: None,
        input_start: 0,
        input_end: 0,
        is_step_marker: false,
        trace_marker_id: None,
        marker_phase: TraceMarkerPhase::None,
        is_explicit_trace_marker: false,
        runtime_kind: Some(TraceEventKind::MatchFail),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 1,
        marker_failure: None,
    };
    let end = NodeTrace {
        node_id: 2,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: NodeTraceStatus::Success,
        label: Some("m".to_string()),
        input_start: 0,
        input_end: 1,
        is_step_marker: true,
        trace_marker_id: Some(1),
        marker_phase: TraceMarkerPhase::End,
        is_explicit_trace_marker: true,
        runtime_kind: Some(TraceEventKind::ParserExit),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 1,
        marker_failure: None,
    };
    let events = vec![start, mid, end];
    let mut map = HashMap::new();
    map.insert(0, 2);
    assert_eq!(
        outcome_for_marker_span(&events, 0, &map),
        MarkerOutcome::Recovered
    );
}

fn paired_marker(success: bool) -> (Vec<NodeTrace>, HashMap<usize, usize>) {
    let mut events = vec![NodeTrace {
        node_id: 0,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: NodeTraceStatus::Enter,
        label: Some("m".to_string()),
        input_start: 0,
        input_end: 0,
        is_step_marker: true,
        trace_marker_id: Some(1),
        marker_phase: TraceMarkerPhase::Start,
        is_explicit_trace_marker: true,
        runtime_kind: Some(TraceEventKind::ParserEnter),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 0,
        marker_failure: None,
    }];
    let end_status = if success {
        NodeTraceStatus::Success
    } else {
        NodeTraceStatus::Fail
    };
    let end_kind = if success {
        TraceEventKind::ParserExit
    } else {
        TraceEventKind::MatchFail
    };
    events.push(NodeTrace {
        node_id: 1,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: end_status,
        label: Some("m".to_string()),
        input_start: 0,
        input_end: 0,
        is_step_marker: true,
        trace_marker_id: Some(1),
        marker_phase: TraceMarkerPhase::End,
        is_explicit_trace_marker: true,
        runtime_kind: Some(end_kind),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 0,
        marker_failure: None,
    });
    let mut map = HashMap::new();
    map.insert(0, 1);
    (events, map)
}
