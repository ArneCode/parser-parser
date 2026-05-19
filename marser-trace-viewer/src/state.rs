//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::collections::HashMap;
use std::fs;

use marser_trace_schema::NodeTrace;
use ratatui::text::{Line, Text};

use crate::marker_index::build_marker_index;
use crate::outcome::{MarkerOutcome, outcome_for_marker_span};

#[derive(Clone)]
pub struct ViewerState {
    pub events: Vec<NodeTrace>,
    pub start_indices: Vec<usize>,
    pub start_to_end: HashMap<usize, usize>,
    pub parent_end_by_start: HashMap<usize, usize>,
    pub current_idx: Option<usize>,
    pub history: Vec<usize>,
    pub history_cursor: Option<usize>,
    pub source_text: Option<String>,
    pub grammar_sources: HashMap<String, String>,
    pub last_step_result: String,
}

impl ViewerState {
    pub fn new(events: Vec<NodeTrace>, source_text: Option<String>) -> Self {
        let marker_index = build_marker_index(&events);

        let mut state = Self {
            events,
            start_indices: marker_index.start_indices,
            start_to_end: marker_index.start_to_end,
            parent_end_by_start: marker_index.parent_end_by_start,
            current_idx: None,
            history: Vec::new(),
            history_cursor: None,
            source_text,
            grammar_sources: HashMap::new(),
            last_step_result: String::new(),
        };
        state.move_to_first_user_start();
        state
    }

    pub fn current_event(&self) -> Option<&NodeTrace> {
        self.current_idx.and_then(|idx| self.events.get(idx))
    }

    pub fn marker_pair_extent_range(&self, source_len: usize) -> Option<(usize, usize)> {
        let start_idx = self.current_idx?;
        let end_idx = self.start_to_end.get(&start_idx).copied()?;
        let start_ev = self.events.get(start_idx)?;
        let end_ev = self.events.get(end_idx)?;
        let lo = start_ev.input_start.min(source_len);
        let hi_raw = end_ev.input_end.min(source_len);
        let hi = if hi_raw > lo {
            hi_raw
        } else {
            (lo + 1).min(source_len)
        };
        if lo >= source_len {
            return None;
        }
        Some((lo, hi))
    }

    fn move_to_first_user_start(&mut self) {
        let first_user = self
            .start_indices
            .iter()
            .copied()
            .find(|idx| {
                self.events[*idx]
                    .usage_loc
                    .as_ref()
                    .is_some_and(|loc| !loc.file.ends_with("/src/lib.rs"))
            })
            .or_else(|| self.start_indices.first().copied());
        if let Some(idx) = first_user {
            self.current_idx = Some(idx);
            self.history.push(idx);
            self.history_cursor = Some(0);
            self.ensure_grammar_loaded();
            self.last_step_result = format!(
                "current: {} ({})",
                self.label_at(idx).unwrap_or("-"),
                self.outcome_for_start(idx).as_str()
            );
        } else {
            self.last_step_result = "current: no trace markers".to_string();
        }
    }

    pub fn ensure_grammar_loaded(&mut self) {
        let Some(rule_file) = self
            .current_event()
            .and_then(|event| event.usage_loc.as_ref().map(|loc| loc.file.clone()))
        else {
            return;
        };
        if self.grammar_sources.contains_key(&rule_file) {
            return;
        }
        if let Ok(text) = fs::read_to_string(&rule_file) {
            self.grammar_sources.insert(rule_file, text);
        }
    }

    fn next_start_after(&self, idx: usize) -> Option<usize> {
        self.start_indices.iter().copied().find(|start| *start > idx)
    }

    pub fn set_current_index(&mut self, idx: usize) {
        if self.current_idx == Some(idx) {
            return;
        }
        if let Some(cursor) = self.history_cursor {
            let next = cursor.saturating_add(1);
            if next < self.history.len() {
                self.history.truncate(next);
            }
        }
        self.history.push(idx);
        self.history_cursor = Some(self.history.len().saturating_sub(1));
        self.current_idx = Some(idx);
        self.ensure_grammar_loaded();
    }

    pub fn step_into(&mut self) {
        let Some(current) = self.current_idx else {
            self.last_step_result = "i: no current marker".to_string();
            return;
        };
        if let Some(next) = self.next_start_after(current) {
            self.set_current_index(next);
            self.last_step_result = format!(
                "i: moved to {} ({})",
                self.current_label().unwrap_or("-"),
                self.current_outcome().as_str()
            );
        } else {
            self.current_idx = None;
            self.last_step_result = "i: parse complete".to_string();
        }
    }

    pub fn step_over(&mut self) {
        let Some(current) = self.current_idx else {
            self.last_step_result = "s: no current marker".to_string();
            return;
        };
        let current_label = self.label_at(current).unwrap_or("-").to_string();
        let current_outcome = self.outcome_for_start(current).as_str();
        let boundary = self.start_to_end.get(&current).copied().unwrap_or(current);
        if let Some(next) = self.next_start_after(boundary) {
            self.set_current_index(next);
            self.last_step_result = format!(
                "s: {} -> {} | now {} ({})",
                current_label,
                current_outcome,
                self.current_label().unwrap_or("-"),
                self.current_outcome().as_str()
            );
        } else {
            self.current_idx = None;
            self.last_step_result =
                format!("s: {} -> {} | parse complete", current_label, current_outcome);
        }
    }

    pub fn step_out(&mut self) {
        let Some(current) = self.current_idx else {
            self.last_step_result = "u: no current marker".to_string();
            return;
        };
        if let Some(end_idx) = self.parent_end_by_start.get(&current).copied() {
            if let Some(next) = self.next_start_after(end_idx) {
                self.set_current_index(next);
                self.last_step_result = format!(
                    "u: moved to {} ({})",
                    self.current_label().unwrap_or("-"),
                    self.current_outcome().as_str()
                );
            } else {
                self.current_idx = None;
                self.last_step_result = "u: parse complete".to_string();
            }
        } else {
            self.current_idx = None;
            self.last_step_result = "u: no parent marker (done)".to_string();
        }
    }

    pub fn go_back(&mut self) {
        let Some(cursor) = self.history_cursor else {
            self.last_step_result = "backspace: no history".to_string();
            return;
        };
        if self.current_idx.is_none() {
            if let Some(idx) = self.history.get(cursor).copied() {
                self.current_idx = Some(idx);
                self.ensure_grammar_loaded();
                self.last_step_result = format!(
                    "backspace: restored {} ({})",
                    self.current_label().unwrap_or("-"),
                    self.current_outcome().as_str()
                );
            }
            return;
        }
        if cursor == 0 {
            self.last_step_result = "backspace: at history start".to_string();
            return;
        }
        let prev = cursor.saturating_sub(1);
        if let Some(idx) = self.history.get(prev).copied() {
            self.current_idx = Some(idx);
            self.history_cursor = Some(prev);
            self.ensure_grammar_loaded();
            self.last_step_result = format!(
                "backspace: moved to {} ({})",
                self.current_label().unwrap_or("-"),
                self.current_outcome().as_str()
            );
        }
    }

    fn label_at(&self, idx: usize) -> Option<&str> {
        self.events.get(idx).and_then(|event| {
            event
                .label
                .as_deref()
                .or(event.rule.as_ref().and_then(|r| r.rule_name.as_deref()))
        })
    }

    pub fn current_label(&self) -> Option<&str> {
        self.current_idx.and_then(|idx| self.label_at(idx))
    }

    pub fn current_outcome(&self) -> MarkerOutcome {
        self.current_idx
            .map(|idx| self.outcome_for_start(idx))
            .unwrap_or(MarkerOutcome::Running)
    }

    fn outcome_for_start(&self, start_idx: usize) -> MarkerOutcome {
        outcome_for_marker_span(&self.events, start_idx, &self.start_to_end)
    }

    pub fn marker_context_text(&self) -> Text<'static> {
        let Some(start_idx) = self.current_idx else {
            return Text::from("—");
        };
        let outcome = self.outcome_for_start(start_idx);
        let Some(start_ev) = self.events.get(start_idx) else {
            return Text::from("—");
        };
        let end_ev = self
            .start_to_end
            .get(&start_idx)
            .and_then(|i| self.events.get(*i));

        let lines: Vec<Line<'static>> = match outcome {
            MarkerOutcome::Success => {
                if let Some(end) = end_ev {
                    let n = end.input_end.saturating_sub(start_ev.input_start);
                    vec![Line::from(format!(
                        "consumed: {n} bytes (input {}..{})",
                        start_ev.input_start, end.input_end
                    ))]
                } else {
                    vec![Line::from("consumed: (no paired end in trace)")]
                }
            }
            MarkerOutcome::Recovered => {
                if let Some(end) = end_ev {
                    let n = end.input_end.saturating_sub(start_ev.input_start);
                    vec![
                        Line::from(
                            "outcome: recovered error — error counters increased inside this marker span",
                        ),
                        Line::from(format!(
                            "consumed: {n} bytes (input {}..{})",
                            start_ev.input_start, end.input_end
                        )),
                    ]
                } else {
                    vec![Line::from(
                        "outcome: recovered error — no paired end; inspect timeline for error-counter growth",
                    )]
                }
            }
            MarkerOutcome::Fail | MarkerOutcome::Error => {
                if let Some(end) = end_ev {
                    if let Some(mf) = end.marker_failure.as_ref() {
                        let expected = if mf.expected.is_empty() {
                            "—".to_string()
                        } else {
                            mf.expected.join(", ")
                        };
                        vec![
                            Line::from(mf.summary.clone()),
                            Line::from(format!(
                                "failure span {}..{} | expected: {expected}",
                                mf.span_start, mf.span_end
                            )),
                        ]
                    } else {
                        let msg = if matches!(outcome, MarkerOutcome::Error) {
                            "fail/error: no snapshot on trace end (regenerate trace)"
                        } else {
                            "fail: no snapshot (old trace or unlabelled soft fail)"
                        };
                        vec![Line::from(msg)]
                    }
                } else {
                    vec![Line::from("fail/error: no paired marker end")]
                }
            }
            MarkerOutcome::Backtrack | MarkerOutcome::Running => {
                let tail = end_ev
                    .map(|e| e.input_end.to_string())
                    .unwrap_or_else(|| "?".to_string());
                vec![Line::from(format!(
                    "outcome: {} | marker input from {}..{tail}",
                    outcome.as_str(),
                    start_ev.input_start
                ))]
            }
        };
        Text::from(lines)
    }
}
