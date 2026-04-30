#![cfg(feature = "parser-trace")]

use std::{fs, time::{SystemTime, UNIX_EPOCH}};

use marser::trace::{
    NodeTrace, NodeTraceKind, NodeTraceStatus, TraceEventKind, TraceMarkerPhase, TraceSession,
    debug_protocol::{Breakpoint, DebugCommand, DebugNotification, DebugSession},
    load::{TraceFormat, load_trace_file},
};

fn sample_events() -> Vec<NodeTrace> {
    vec![
        NodeTrace {
            node_id: 0,
            parent_node_id: None,
            usage_loc: None,
            definition_loc: None,
            kind: NodeTraceKind::Runtime,
            status: NodeTraceStatus::Enter,
            label: Some("root".to_string()),
            input_start: 0,
            input_end: 0,
            is_step_marker: false,
            trace_marker_id: None,
            marker_phase: TraceMarkerPhase::None,
            is_explicit_trace_marker: false,
            runtime_kind: Some(TraceEventKind::ChoiceStart),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
        NodeTrace {
            node_id: 1,
            parent_node_id: Some(0),
            usage_loc: None,
            definition_loc: None,
            kind: NodeTraceKind::CaptureBoundary,
            status: NodeTraceStatus::Enter,
            label: Some("capture".to_string()),
            input_start: 0,
            input_end: 0,
            is_step_marker: true,
            trace_marker_id: Some(10),
            marker_phase: TraceMarkerPhase::Start,
            is_explicit_trace_marker: true,
            runtime_kind: Some(TraceEventKind::CaptureEnter),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
        NodeTrace {
            node_id: 2,
            parent_node_id: Some(0),
            usage_loc: None,
            definition_loc: None,
            kind: NodeTraceKind::Runtime,
            status: NodeTraceStatus::Success,
            label: Some("arm".to_string()),
            input_start: 0,
            input_end: 1,
            is_step_marker: false,
            trace_marker_id: None,
            marker_phase: TraceMarkerPhase::None,
            is_explicit_trace_marker: false,
            runtime_kind: Some(TraceEventKind::MatchSuccess),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
        NodeTrace {
            node_id: 3,
            parent_node_id: Some(0),
            usage_loc: None,
            definition_loc: None,
            kind: NodeTraceKind::CaptureBoundary,
            status: NodeTraceStatus::Success,
            label: Some("capture".to_string()),
            input_start: 0,
            input_end: 1,
            is_step_marker: true,
            trace_marker_id: Some(10),
            marker_phase: TraceMarkerPhase::End,
            is_explicit_trace_marker: true,
            runtime_kind: Some(TraceEventKind::CaptureExit),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
        NodeTrace {
            node_id: 4,
            parent_node_id: None,
            usage_loc: None,
            definition_loc: None,
            kind: NodeTraceKind::Runtime,
            status: NodeTraceStatus::Success,
            label: Some("done".to_string()),
            input_start: 0,
            input_end: 1,
            is_step_marker: true,
            trace_marker_id: Some(11),
            marker_phase: TraceMarkerPhase::Start,
            is_explicit_trace_marker: true,
            runtime_kind: Some(TraceEventKind::MatchSuccess),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
    ]
}

fn temp_file_path(ext: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("marser_trace_{stamp}.{ext}"))
}

#[test]
fn loads_json_trace_file() {
    let session = TraceSession::from_events(sample_events());
    let path = temp_file_path("json");
    fs::write(&path, serde_json::to_string(session.events()).unwrap()).unwrap();
    let loaded = load_trace_file(&path, Some(TraceFormat::Json)).unwrap();
    fs::remove_file(&path).ok();
    assert_eq!(loaded.events().len(), 5);
    assert_eq!(loaded.events()[2].runtime_kind, Some(TraceEventKind::MatchSuccess));
}

#[test]
fn loads_jsonl_trace_file() {
    let path = temp_file_path("jsonl");
    let mut body = String::new();
    for event in sample_events() {
        body.push_str(&serde_json::to_string(&event).unwrap());
        body.push('\n');
    }
    fs::write(&path, body).unwrap();
    let loaded = load_trace_file(&path, Some(TraceFormat::Jsonl)).unwrap();
    fs::remove_file(&path).ok();
    assert_eq!(loaded.events().len(), 5);
    assert_eq!(loaded.events()[0].runtime_kind, Some(TraceEventKind::ChoiceStart));
}

#[test]
fn replay_debug_session_steps_and_breakpoints() {
    let trace = TraceSession::from_events(sample_events());
    let mut debug = DebugSession::from_trace(&trace);
    match debug.handle_command(DebugCommand::Step) {
        Some(DebugNotification::CurrentEvent(event)) => assert_eq!(event.node_id, 1),
        other => panic!("unexpected notification: {other:?}"),
    }
    debug.handle_command(DebugCommand::SetBreakpoint(Breakpoint::EventKind(
        TraceEventKind::MatchSuccess,
    )));
    match debug.handle_command(DebugCommand::Continue) {
        Some(DebugNotification::BreakpointHit { event, .. }) => {
            assert_eq!(event.runtime_kind, Some(TraceEventKind::MatchSuccess));
        }
        other => panic!("unexpected notification: {other:?}"),
    }
}

#[test]
fn replay_debug_session_step_modes() {
    let trace = TraceSession::from_events(sample_events());

    let mut debug = DebugSession::from_trace(&trace);
    match debug.handle_command(DebugCommand::StepOver) {
        Some(DebugNotification::CurrentEvent(event)) => assert_eq!(event.node_id, 1),
        other => panic!("unexpected notification: {other:?}"),
    }

    let mut debug = DebugSession::from_trace(&trace);
    let _ = debug.handle_command(DebugCommand::StepInto);
    match debug.handle_command(DebugCommand::StepOut) {
        Some(DebugNotification::ParseComplete) => {}
        other => panic!("unexpected notification: {other:?}"),
    }
}

