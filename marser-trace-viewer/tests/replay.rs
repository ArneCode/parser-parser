use marser_trace_schema::{
    NodeTrace, NodeTraceKind, NodeTraceStatus, TraceEventKind, TraceMarkerPhase, TraceSession,
};
use marser_trace_viewer::replay::{
    ReplayBreakpoint, ReplayCommand, ReplayNotification, ReplaySession,
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
            runtime_kind: Some(TraceEventKind::ParserEnter),
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
            runtime_kind: Some(TraceEventKind::ParserEnter),
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
            runtime_kind: Some(TraceEventKind::ParserExit),
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
            runtime_kind: Some(TraceEventKind::MatchFail),
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
            runtime_kind: Some(TraceEventKind::ParserExit),
            rule: None,
            error_sink_len: 0,
            error_stack_len: 0,
            marker_failure: None,
        },
    ]
}

#[test]
fn replay_session_steps_and_breakpoints() {
    let trace = TraceSession::from_events(sample_events());
    let mut debug = ReplaySession::from_trace(&trace);
    match debug.handle_command(ReplayCommand::StepOver) {
        Some(ReplayNotification::CurrentEvent(event)) => assert_eq!(event.node_id, 1),
        other => panic!("unexpected notification: {other:?}"),
    }
    debug.handle_command(ReplayCommand::SetBreakpoint(ReplayBreakpoint::EventKind(
        TraceEventKind::ParserExit,
    )));
    match debug.handle_command(ReplayCommand::Continue) {
        Some(ReplayNotification::BreakpointHit { event, .. }) => {
            assert_eq!(event.runtime_kind, Some(TraceEventKind::ParserExit));
        }
        other => panic!("unexpected notification: {other:?}"),
    }
}

#[test]
fn replay_session_step_modes() {
    let trace = TraceSession::from_events(sample_events());

    let mut debug = ReplaySession::from_trace(&trace);
    match debug.handle_command(ReplayCommand::StepOver) {
        Some(ReplayNotification::CurrentEvent(event)) => assert_eq!(event.node_id, 1),
        other => panic!("unexpected notification: {other:?}"),
    }

    let mut debug = ReplaySession::from_trace(&trace);
    let _ = debug.handle_command(ReplayCommand::StepInto);
    match debug.handle_command(ReplayCommand::StepOut) {
        Some(ReplayNotification::ParseComplete) => {}
        other => panic!("unexpected notification: {other:?}"),
    }
}
