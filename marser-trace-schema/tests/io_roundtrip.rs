//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use marser_trace_schema::{
    NodeTrace, NodeTraceKind, NodeTraceStatus, TraceEventKind, TraceMarkerPhase, TraceSession,
    load_json, check_trace_version,
};

#[test]
fn json_object_envelope_requires_supported_version() {
    assert!(check_trace_version(Some(2)).is_ok());
    assert!(check_trace_version(Some(99)).is_err());
}

#[test]
fn load_json_accepts_legacy_array_without_version() {
    let session = TraceSession::from_events(vec![NodeTrace {
        node_id: 0,
        parent_node_id: None,
        usage_loc: None,
        definition_loc: None,
        kind: NodeTraceKind::Runtime,
        status: NodeTraceStatus::Enter,
        label: None,
        input_start: 0,
        input_end: 0,
        is_step_marker: false,
        trace_marker_id: None,
        marker_phase: TraceMarkerPhase::None,
        is_explicit_trace_marker: false,
        runtime_kind: Some(TraceEventKind::ParserEnter),
        rule: None,
        error_sink_len: 0,
        error_stack_len: 0,
        marker_failure: None,
    }]);
    let json = serde_json::to_string(session.events()).unwrap();
    let loaded = load_json(json.as_bytes()).unwrap();
    assert_eq!(loaded.events().len(), 1);
}
