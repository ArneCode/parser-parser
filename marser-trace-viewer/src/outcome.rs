//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use marser_trace_schema::{NodeTrace, NodeTraceStatus, TraceEventKind, TraceMarkerPhase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerOutcome {
    Running,
    Success,
    Recovered,
    Fail,
    Error,
    Backtrack,
}

impl MarkerOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Recovered => "recovered error",
            Self::Fail | Self::Running | Self::Backtrack => "fail",
            Self::Error => "error",
        }
    }
}

pub fn outcome_for_marker_span(events: &[NodeTrace], start_idx: usize, start_to_end: &std::collections::HashMap<usize, usize>) -> MarkerOutcome {
    if let Some(end_idx) = start_to_end.get(&start_idx).copied() {
        let start = &events[start_idx];
        let mut saw_error = false;
        for event in &events[(start_idx + 1)..end_idx] {
            if matches!(
                event.runtime_kind,
                Some(TraceEventKind::MatchHardError)
            ) {
                saw_error = true;
                break;
            }
        }
        if saw_error {
            return MarkerOutcome::Error;
        }
        let mut max_error_stack_len = start.error_stack_len;
        for event in &events[(start_idx + 1)..end_idx] {
            max_error_stack_len = max_error_stack_len.max(event.error_stack_len);
        }
        let end = &events[end_idx];
        max_error_stack_len = max_error_stack_len.max(end.error_stack_len);
        if matches!(
            end.runtime_kind,
            Some(TraceEventKind::MatchHardError)
        ) {
            return MarkerOutcome::Error;
        }
        let recovered_by_counters = max_error_stack_len > start.error_stack_len;
        if recovered_by_counters {
            return MarkerOutcome::Recovered;
        }
        return match end.status {
            NodeTraceStatus::Success => MarkerOutcome::Success,
            NodeTraceStatus::Fail => MarkerOutcome::Fail,
            NodeTraceStatus::Backtrack => MarkerOutcome::Backtrack,
            NodeTraceStatus::Enter => MarkerOutcome::Running,
        };
    }

    let window_end_exclusive = next_start_index(events, start_idx).unwrap_or_else(|| events.len());
    let mut saw_error = false;
    let mut saw_fail = false;
    let mut saw_success_end = false;
    let mut saw_backtrack = false;
    let start = &events[start_idx];
    let mut max_error_stack_len = start.error_stack_len;
    for event in &events[start_idx..window_end_exclusive] {
        if matches!(
            event.runtime_kind,
            Some(TraceEventKind::MatchHardError)
        ) {
            saw_error = true;
        }
        max_error_stack_len = max_error_stack_len.max(event.error_stack_len);
        match event.status {
            NodeTraceStatus::Backtrack => saw_backtrack = true,
            NodeTraceStatus::Fail => saw_fail = true,
            NodeTraceStatus::Success
                if event.is_explicit_trace_marker
                    && matches!(event.marker_phase, TraceMarkerPhase::End) =>
            {
                saw_success_end = true;
            }
            _ => {}
        }
    }
    if saw_error {
        MarkerOutcome::Error
    } else if saw_fail {
        MarkerOutcome::Fail
    } else if max_error_stack_len > start.error_stack_len {
        MarkerOutcome::Recovered
    } else if saw_success_end {
        MarkerOutcome::Success
    } else if saw_backtrack {
        MarkerOutcome::Backtrack
    } else {
        MarkerOutcome::Running
    }
}

fn next_start_index(events: &[NodeTrace], after: usize) -> Option<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(idx, e)| {
            *idx > after && e.is_explicit_trace_marker && matches!(e.marker_phase, TraceMarkerPhase::Start)
        })
        .map(|(idx, _)| idx)
        .next()
}
