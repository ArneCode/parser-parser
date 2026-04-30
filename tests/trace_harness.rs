#![cfg(feature = "parser-trace")]

use marser::{
    label::WithTrace,
    matcher::{MatcherCombinator, many},
    one_of::one_of,
    parser::Parser,
    trace::{NodeTraceStatus, TraceEventKind, TraceMarkerPhase},
};

fn tiny_parser<'src>() -> impl Parser<'src, &'src str, Output = &'src str> {
    one_of(("ab".to("ab"), "ac".to("ac")))
}

#[test]
fn trace_collects_choice_events() {
    let (_out, _errors, trace) = marser::parse_with_trace(tiny_parser(), "ac").unwrap();
    let kinds: Vec<_> = trace
        .events()
        .iter()
        .filter_map(|event| event.runtime_kind.as_ref())
        .collect();

    assert!(kinds.contains(&&TraceEventKind::ChoiceStart));
    assert!(kinds.contains(&&TraceEventKind::ChoiceArmStart));
    assert!(kinds.contains(&&TraceEventKind::ChoiceArmFail));
    assert!(kinds.contains(&&TraceEventKind::ChoiceArmSuccess));
    assert!(kinds.contains(&&TraceEventKind::CaptureEnter));
    assert!(kinds.contains(&&TraceEventKind::CaptureExit));
}

#[test]
fn trace_event_ids_are_monotonic() {
    let (_out, _errors, trace) = marser::parse_with_trace(tiny_parser(), "ab").unwrap();
    let mut expected_id = 0u64;
    for event in trace.events() {
        assert_eq!(event.node_id, expected_id);
        expected_id += 1;
    }
}

#[test]
fn trace_text_renderers_produce_output() {
    let (_out, _errors, trace) = marser::parse_with_trace(tiny_parser(), "ab").unwrap();
    assert!(!trace.to_text_tree().is_empty());
    assert!(!trace.to_timeline().is_empty());
}

#[test]
fn trace_events_include_rule_identity_metadata() {
    let (_out, _errors, trace) = marser::parse_with_trace(tiny_parser(), "ac").unwrap();
    let with_rule = trace
        .events()
        .iter()
        .filter_map(|event| event.rule.as_ref())
        .collect::<Vec<_>>();
    assert!(!with_rule.is_empty(), "expected at least one event with rule metadata");
    assert!(
        with_rule
            .iter()
            .any(|rule| rule.rule_file.ends_with("tests/trace_harness.rs")),
        "expected user grammar callsite metadata in trace"
    );
}

#[test]
fn same_rule_reuses_rule_id_within_parse() {
    let (_out, _errors, trace) = marser::parse_with_trace(tiny_parser(), "ac").unwrap();
    let first_rule_id = trace
        .events()
        .iter()
        .find_map(|event| event.rule.as_ref().map(|rule| rule.rule_id))
        .expect("expected rule metadata");
    let count = trace
        .events()
        .iter()
        .filter(|event| event.rule.as_ref().is_some_and(|rule| rule.rule_id == first_rule_id))
        .count();
    assert!(count > 1, "expected same rule id to appear multiple times");
}

#[test]
fn rule_id_sequence_is_deterministic_across_runs() {
    let (_out, _errors, trace_a) = marser::parse_with_trace(tiny_parser(), "ac").unwrap();
    let (_out, _errors, trace_b) = marser::parse_with_trace(tiny_parser(), "ac").unwrap();
    let seq_a = trace_a
        .events()
        .iter()
        .map(|event| event.rule.as_ref().map(|rule| rule.rule_id))
        .collect::<Vec<_>>();
    let seq_b = trace_b
        .events()
        .iter()
        .map(|event| event.rule.as_ref().map(|rule| rule.rule_id))
        .collect::<Vec<_>>();
    assert_eq!(seq_a, seq_b);
}

#[test]
fn depth_only_changes_at_capture_boundaries() {
    let parser = many('a').to(());
    let (_out, _errors, trace) = marser::parse_with_trace(parser, "aaa").unwrap();
    let match_enter_depths = trace
        .events()
        .iter()
        .filter(|event| matches!(event.runtime_kind, Some(TraceEventKind::MatchEnter)))
        .map(|event| event.parent_node_id)
        .collect::<Vec<_>>();
    assert!(!match_enter_depths.is_empty());
    let first = match_enter_depths[0];
    assert!(
        match_enter_depths.iter().all(|depth| *depth == first),
        "expected matcher depth to remain constant within a capture"
    );
}

#[test]
fn explicit_trace_markers_are_paired_and_ordered() {
    let parser = one_of((
        "ab".to("ab").trace_with_label("first arm"),
        "ac".to("ac").trace_with_label("second arm"),
    ));
    let (_out, _errors, trace) = marser::parse_with_trace(parser, "ac").unwrap();
    let explicit = trace
        .events()
        .iter()
        .filter(|event| event.is_explicit_trace_marker)
        .collect::<Vec<_>>();
    assert!(!explicit.is_empty());

    let start_labels = explicit
        .iter()
        .filter(|event| matches!(event.marker_phase, TraceMarkerPhase::Start))
        .filter_map(|event| event.label.clone())
        .collect::<Vec<_>>();
    assert!(
        start_labels.windows(2).any(|pair| pair == ["first arm", "second arm"]),
        "expected explicit marker starts to preserve trace call order"
    );

    for marker_id in explicit.iter().filter_map(|event| event.trace_marker_id) {
        let phases = explicit
            .iter()
            .filter(|event| event.trace_marker_id == Some(marker_id))
            .map(|event| &event.marker_phase)
            .collect::<Vec<_>>();
        assert!(
            phases.contains(&&TraceMarkerPhase::Start) && phases.contains(&&TraceMarkerPhase::End),
            "expected marker {marker_id} to have both start and end phases"
        );
    }
}

#[test]
fn explicit_trace_marker_end_reflects_inner_soft_fail() {
    let parser = one_of((
        "zz"
            .to("zz")
            .trace_with_label("no match"),
        "ab".to("ab"),
    ));
    let (_out, _errors, trace) = marser::parse_with_trace(parser, "ab").unwrap();
    let end = trace
        .events()
        .iter()
        .find(|e| {
            e.is_explicit_trace_marker
                && matches!(e.marker_phase, TraceMarkerPhase::End)
                && e.label.as_deref() == Some("no match")
        })
        .expect("trace end for failed arm");
    assert_eq!(end.runtime_kind, Some(TraceEventKind::MatchFail));
    assert_eq!(end.status, NodeTraceStatus::Fail);
}

#[test]
fn explicit_trace_marker_end_reflects_inner_success() {
    let parser = "ab".to("ab").trace_with_label("ok");
    let (_out, _errors, trace) = marser::parse_with_trace(parser, "ab").unwrap();
    let end = trace
        .events()
        .iter()
        .find(|e| e.is_explicit_trace_marker && matches!(e.marker_phase, TraceMarkerPhase::End))
        .expect("trace end");
    assert_eq!(end.runtime_kind, Some(TraceEventKind::ParserExit));
    assert_eq!(end.status, NodeTraceStatus::Success);
    assert!(end.marker_failure.is_none());
}

#[test]
fn explicit_trace_marker_end_includes_failure_snapshot_on_soft_fail() {
    let parser = one_of((
        "zz".to("zz").trace_with_label("no match"),
        "ab".to("ab"),
    ));
    let (_out, _errors, trace) = marser::parse_with_trace(parser, "ab").unwrap();
    let end = trace
        .events()
        .iter()
        .find(|e| {
            e.is_explicit_trace_marker
                && matches!(e.marker_phase, TraceMarkerPhase::End)
                && e.label.as_deref() == Some("no match")
        })
        .expect("trace end");
    let mf = end
        .marker_failure
        .as_ref()
        .expect("expected marker_failure snapshot on failed trace end");
    assert!(!mf.summary.is_empty());
}


