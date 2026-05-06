//! In-process replay / stepping over a trace event list (used by the TUI and tests).

use std::collections::HashMap;

use marser_trace_schema::{NodeTrace, TraceEventKind, TraceMarkerPhase, TraceSession};
use crate::marker_index::build_marker_index;

#[derive(Clone, Debug)]
pub enum ReplayCommand {
    Start,
    StepInto,
    StepOver,
    StepOut,
    Continue,
    Pause,
    SetBreakpoint(ReplayBreakpoint),
    ClearBreakpoints,
    InspectPosition,
}

#[derive(Clone, Debug)]
pub enum ReplayBreakpoint {
    EventKind(TraceEventKind),
    Label(String),
    RuleId(u64),
    PositionRange { start: usize, end: usize },
}

#[derive(Clone, Debug)]
pub enum ReplayNotification {
    CurrentEvent(NodeTrace),
    BreakpointHit {
        event: NodeTrace,
        reason: ReplayBreakpoint,
    },
    ParseComplete,
}

#[derive(Clone, Debug)]
pub struct ReplaySession {
    events: Vec<NodeTrace>,
    cursor: usize,
    breakpoints: Vec<ReplayBreakpoint>,
    marker_start_to_end: HashMap<usize, usize>,
    marker_start_indices: Vec<usize>,
    enclosing_parent_end_by_index: Vec<Option<usize>>,
}

impl ReplaySession {
    pub fn from_trace(session: &TraceSession) -> Self {
        let events = session.events().to_vec();
        let marker_index = build_marker_index(&events);
        Self {
            events,
            cursor: 0,
            breakpoints: Vec::new(),
            marker_start_to_end: marker_index.start_to_end,
            marker_start_indices: marker_index.start_indices,
            enclosing_parent_end_by_index: marker_index.enclosing_parent_end_by_index,
        }
    }

    pub fn handle_command(&mut self, command: ReplayCommand) -> Option<ReplayNotification> {
        match command {
            ReplayCommand::Start => self.current_event_notification(),
            ReplayCommand::StepInto => self.step_into(),
            ReplayCommand::StepOver => self.step_over(),
            ReplayCommand::StepOut => self.step_out(),
            ReplayCommand::Continue => self.run_until_breakpoint_or_end(),
            ReplayCommand::Pause => self.current_event_notification(),
            ReplayCommand::SetBreakpoint(bp) => {
                self.breakpoints.push(bp);
                self.current_event_notification()
            }
            ReplayCommand::ClearBreakpoints => {
                self.breakpoints.clear();
                self.current_event_notification()
            }
            ReplayCommand::InspectPosition => self.current_event_notification(),
        }
    }

    fn current_event_notification(&self) -> Option<ReplayNotification> {
        if self.cursor < self.events.len() {
            Some(ReplayNotification::CurrentEvent(
                self.events[self.cursor].clone(),
            ))
        } else {
            Some(ReplayNotification::ParseComplete)
        }
    }

    fn step_into(&mut self) -> Option<ReplayNotification> {
        if let Some(current) = self.events.get(self.cursor)
            && current.is_explicit_trace_marker
            && matches!(current.marker_phase, TraceMarkerPhase::Start)
            && current.definition_loc.is_some()
            && current.definition_loc.as_ref() != current.usage_loc.as_ref()
        {
            return Some(ReplayNotification::CurrentEvent(current.clone()));
        }
        if let Some(next_idx) = self.next_marker_start_after(self.cursor) {
            self.cursor = next_idx;
            return Some(ReplayNotification::CurrentEvent(self.events[next_idx].clone()));
        }
        self.cursor = self.events.len();
        Some(ReplayNotification::ParseComplete)
    }

    fn step_over(&mut self) -> Option<ReplayNotification> {
        let Some(current) = self.events.get(self.cursor) else {
            return Some(ReplayNotification::ParseComplete);
        };
        let target_start = if current.is_explicit_trace_marker
            && matches!(current.marker_phase, TraceMarkerPhase::Start)
        {
            if let Some(end_idx) = self.marker_start_to_end.get(&self.cursor).copied() {
                self.next_marker_start_after(end_idx)
            } else {
                self.next_marker_start_after(self.cursor)
            }
        } else {
            self.next_marker_start_after(self.cursor)
        };
        if let Some(idx) = target_start {
            self.cursor = idx;
            return Some(ReplayNotification::CurrentEvent(self.events[idx].clone()));
        }
        self.cursor = self.events.len();
        Some(ReplayNotification::ParseComplete)
    }

    fn step_out(&mut self) -> Option<ReplayNotification> {
        let current_idx = self.cursor;
        let Some(_current) = self.events.get(current_idx) else {
            return Some(ReplayNotification::ParseComplete);
        };
        if let Some(end_idx) = self
            .enclosing_parent_end_by_index
            .get(current_idx)
            .copied()
            .flatten()
            && let Some(next_idx) = self.next_marker_start_after(end_idx)
        {
            self.cursor = next_idx;
            return Some(ReplayNotification::CurrentEvent(self.events[next_idx].clone()));
        }
        self.cursor = self.events.len();
        Some(ReplayNotification::ParseComplete)
    }

    fn run_until_breakpoint_or_end(&mut self) -> Option<ReplayNotification> {
        let mut idx = self.cursor.saturating_add(1);
        while idx < self.events.len() {
            let event = self.events[idx].clone();
            self.cursor = idx;
            idx = idx.saturating_add(1);
            if let Some(reason) = self
                .breakpoints
                .iter()
                .find(|bp| Self::matches_breakpoint(bp, &event))
                .cloned()
            {
                return Some(ReplayNotification::BreakpointHit { event, reason });
            }
        }
        self.cursor = self.events.len();
        Some(ReplayNotification::ParseComplete)
    }

    fn matches_breakpoint(bp: &ReplayBreakpoint, event: &NodeTrace) -> bool {
        match bp {
            ReplayBreakpoint::EventKind(kind) => event.runtime_kind.as_ref().is_some_and(|runtime_kind| {
                std::mem::discriminant(kind) == std::mem::discriminant(runtime_kind)
            }),
            ReplayBreakpoint::Label(label) => event.label.as_ref() == Some(label),
            ReplayBreakpoint::RuleId(rule_id) => event
                .rule
                .as_ref()
                .is_some_and(|rule| &rule.rule_id == rule_id),
            ReplayBreakpoint::PositionRange { start, end } => {
                event.input_start >= *start && event.input_end <= *end
            }
        }
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    fn next_marker_start_after(&self, idx: usize) -> Option<usize> {
        self.marker_start_indices
            .iter()
            .copied()
            .find(|marker_idx| *marker_idx > idx)
    }
}
