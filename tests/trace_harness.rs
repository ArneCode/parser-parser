//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

#![cfg(feature = "parser-trace")]

use marser::{
    matcher::{MatcherCombinator, many},
    one_of::one_of,
    parse_with_trace, parse_with_trace_to_file,
    parser::Parser,
    trace::{NodeTraceStatus, TraceEventKind, TraceFormat, TraceMarkerPhase, WithTrace},
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn tiny_parser<'src>() -> impl Parser<'src, &'src str, Output = &'src str> + Clone {
    one_of(("ab".to("ab"), "ac".to("ac")))
}

#[test]
fn trace_collects_explicit_marker_events() {
    let parser = one_of((
        "ab".to("ab").trace_with_label("first arm"),
        "ac".to("ac").trace_with_label("second arm"),
    ));
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ac").unwrap();
    let kinds: Vec<_> = trace
        .events()
        .iter()
        .filter(|event| event.is_explicit_trace_marker)
        .filter_map(|event| event.runtime_kind.as_ref())
        .collect();
    assert!(kinds.contains(&&TraceEventKind::ParserEnter));
    assert!(kinds.contains(&&TraceEventKind::ParserExit));
    assert!(kinds.contains(&&TraceEventKind::MatchFail));
}

#[test]
fn trace_event_ids_are_monotonic() {
    let (_out, _errors, trace) = parse_with_trace(tiny_parser(), "ab").unwrap();
    for (expected_id, event) in trace.events().iter().enumerate() {
        assert_eq!(event.node_id, expected_id as u64);
    }
}

#[test]
fn trace_events_include_rule_identity_metadata() {
    let parser = "ab".to("ab").trace_with_label("ok");
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ab").unwrap();
    let with_rule = trace
        .events()
        .iter()
        .filter_map(|event| event.rule.as_ref())
        .collect::<Vec<_>>();
    assert!(
        !with_rule.is_empty(),
        "expected at least one event with rule metadata"
    );
    assert!(
        with_rule
            .iter()
            .any(|rule| rule.rule_file.ends_with("tests/trace_harness.rs")),
        "expected user grammar callsite metadata in trace"
    );
}

#[test]
fn same_rule_reuses_rule_id_within_parse() {
    let parser = one_of((
        "ab".to("ab").trace_with_label("same"),
        "ac".to("ac").trace_with_label("same"),
    ));
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ac").unwrap();
    let first_rule_id = trace
        .events()
        .iter()
        .find_map(|event| event.rule.as_ref().map(|rule| rule.rule_id))
        .expect("expected rule metadata");
    let count = trace
        .events()
        .iter()
        .filter(|event| {
            event
                .rule
                .as_ref()
                .is_some_and(|rule| rule.rule_id == first_rule_id)
        })
        .count();
    assert!(count > 1, "expected same rule id to appear multiple times");
}

#[test]
fn rule_id_sequence_is_deterministic_across_runs() {
    let (_out, _errors, trace_a) = parse_with_trace(tiny_parser(), "ac").unwrap();
    let (_out, _errors, trace_b) = parse_with_trace(tiny_parser(), "ac").unwrap();
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
    let parser = many('a').to(()).trace_with_label("many a");
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "aaa").unwrap();
    let starts = trace
        .events()
        .iter()
        .filter(|event| {
            event.is_explicit_trace_marker && matches!(event.marker_phase, TraceMarkerPhase::Start)
        })
        .collect::<Vec<_>>();
    let ends = trace
        .events()
        .iter()
        .filter(|event| {
            event.is_explicit_trace_marker && matches!(event.marker_phase, TraceMarkerPhase::End)
        })
        .collect::<Vec<_>>();
    assert!(!starts.is_empty());
    assert_eq!(
        starts.len(),
        ends.len(),
        "explicit starts/ends should stay paired"
    );
}

#[test]
fn explicit_trace_markers_are_paired_and_ordered() {
    let parser = one_of((
        "ab".to("ab").trace_with_label("first arm"),
        "ac".to("ac").trace_with_label("second arm"),
    ));
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ac").unwrap();
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
        start_labels
            .windows(2)
            .any(|pair| pair == ["first arm", "second arm"]),
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
    let parser = one_of(("zz".to("zz").trace_with_label("no match"), "ab".to("ab")));
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ab").unwrap();
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
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ab").unwrap();
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
    let parser = one_of(("zz".to("zz").trace_with_label("no match"), "ab".to("ab")));
    let (_out, _errors, trace) = parse_with_trace(parser.clone(), "ab").unwrap();
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

#[test]
fn parse_with_trace_to_file_writes_trace_on_error() {
    let parser = "ab".to("ab").trace_with_label("must fail");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let trace_path = std::env::temp_dir().join(format!("marser-trace-error-{nanos}.json"));

    let result = parse_with_trace_to_file(parser.clone(), "ac", &trace_path, TraceFormat::Json);
    assert!(result.is_err(), "parse should fail for mismatched input");

    let trace_text = fs::read_to_string(&trace_path).expect("trace file should be written");
    assert!(
        trace_text.contains("\"nodes\""),
        "expected trace JSON payload to contain nodes"
    );
    assert!(
        trace_text.contains("\"source_text\":\"ac\""),
        "expected trace JSON payload to embed original source text"
    );

    let _ = fs::remove_file(trace_path);
}
