//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::collections::HashMap;

use marser_trace_schema::{NodeTrace, TraceMarkerPhase};

pub struct MarkerIndex {
    pub start_indices: Vec<usize>,
    pub start_to_end: HashMap<usize, usize>,
    pub parent_end_by_start: HashMap<usize, usize>,
    pub enclosing_parent_end_by_index: Vec<Option<usize>>,
}

pub fn build_marker_index(events: &[NodeTrace]) -> MarkerIndex {
    let start_indices = events
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.is_explicit_trace_marker && matches!(e.marker_phase, TraceMarkerPhase::Start)
        })
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();

    let mut starts_by_marker = HashMap::new();
    let mut start_to_end = HashMap::new();
    for (idx, event) in events.iter().enumerate() {
        if !event.is_explicit_trace_marker {
            continue;
        }
        match (event.trace_marker_id, &event.marker_phase) {
            (Some(id), TraceMarkerPhase::Start) => {
                starts_by_marker.insert(id, idx);
            }
            (Some(id), TraceMarkerPhase::End) => {
                if let Some(start_idx) = starts_by_marker.get(&id).copied() {
                    start_to_end.insert(start_idx, idx);
                }
            }
            _ => {}
        }
    }

    let mut parent_end_by_start = HashMap::new();
    let mut enclosing_parent_end_by_index = vec![None; events.len()];
    let mut active_starts: Vec<usize> = Vec::new();

    for (idx, _event) in events.iter().enumerate() {
        while let Some(last_start) = active_starts.last().copied() {
            let Some(last_end) = start_to_end.get(&last_start).copied() else {
                active_starts.pop();
                continue;
            };
            if idx > last_end {
                active_starts.pop();
            } else {
                break;
            }
        }

        enclosing_parent_end_by_index[idx] = active_starts
            .last()
            .and_then(|start| start_to_end.get(start).copied());

        if start_indices.binary_search(&idx).is_ok() {
            if let Some(parent_end) = enclosing_parent_end_by_index[idx] {
                parent_end_by_start.insert(idx, parent_end);
            }
            if start_to_end.contains_key(&idx) {
                active_starts.push(idx);
            }
        }
    }

    MarkerIndex {
        start_indices,
        start_to_end,
        parent_end_by_start,
        enclosing_parent_end_by_index,
    }
}
