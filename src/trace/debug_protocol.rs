use std::collections::HashMap;

use crate::trace::{NodeTrace, TraceEventKind, TraceMarkerPhase, TraceSession};

#[derive(Clone, Debug)]
pub enum DebugCommand {
    Start,
    Step,
    StepInto,
    StepOver,
    StepOut,
    Continue,
    Pause,
    SetBreakpoint(Breakpoint),
    ClearBreakpoints,
    InspectPosition,
}

#[derive(Clone, Debug)]
pub enum Breakpoint {
    EventKind(TraceEventKind),
    Label(String),
    RuleId(u64),
    PositionRange { start: usize, end: usize },
}

#[derive(Clone, Debug)]
pub enum DebugNotification {
    CurrentEvent(NodeTrace),
    BreakpointHit { event: NodeTrace, reason: Breakpoint },
    ParseComplete,
}

#[derive(Clone, Debug)]
pub struct DebugSession {
    events: Vec<NodeTrace>,
    cursor: usize,
    breakpoints: Vec<Breakpoint>,
    force_definition_view: bool,
    focused_scope: Option<u64>,
    marker_start_to_end: HashMap<usize, usize>,
    marker_start_indices: Vec<usize>,
}

impl DebugSession {
    pub fn from_trace(session: &TraceSession) -> Self {
        let events = session.events().to_vec();
        let mut marker_starts_by_id: HashMap<u64, usize> = HashMap::new();
        let mut marker_start_to_end = HashMap::new();
        let mut marker_start_indices = Vec::new();
        for (idx, event) in events.iter().enumerate() {
            if !event.is_explicit_trace_marker {
                continue;
            }
            match (event.trace_marker_id, &event.marker_phase) {
                (Some(marker_id), TraceMarkerPhase::Start) => {
                    marker_starts_by_id.insert(marker_id, idx);
                    marker_start_indices.push(idx);
                }
                (Some(marker_id), TraceMarkerPhase::End) => {
                    if let Some(start_idx) = marker_starts_by_id.get(&marker_id).copied() {
                        marker_start_to_end.insert(start_idx, idx);
                    }
                }
                _ => {}
            }
        }
        Self {
            events,
            cursor: 0,
            breakpoints: Vec::new(),
            force_definition_view: false,
            focused_scope: None,
            marker_start_to_end,
            marker_start_indices,
        }
    }

    pub fn handle_command(&mut self, command: DebugCommand) -> Option<DebugNotification> {
        match command {
            DebugCommand::Start => self.current_event_notification(),
            DebugCommand::Step => self.step_over(),
            DebugCommand::StepInto => self.step_into(),
            DebugCommand::StepOver => self.step_over(),
            DebugCommand::StepOut => self.step_out(),
            DebugCommand::Continue => self.run_until_breakpoint_or_end(),
            DebugCommand::Pause => self.current_event_notification(),
            DebugCommand::SetBreakpoint(bp) => {
                self.breakpoints.push(bp);
                self.current_event_notification()
            }
            DebugCommand::ClearBreakpoints => {
                self.breakpoints.clear();
                self.current_event_notification()
            }
            DebugCommand::InspectPosition => self.current_event_notification(),
        }
    }

    fn current_event_notification(&self) -> Option<DebugNotification> {
        self.events
            .get(self.cursor)
            .cloned()
            .map(DebugNotification::CurrentEvent)
            .or(Some(DebugNotification::ParseComplete))
    }

    fn step_into(&mut self) -> Option<DebugNotification> {
        if let Some(current) = self.events.get(self.cursor)
            && current.is_explicit_trace_marker
            && matches!(current.marker_phase, TraceMarkerPhase::Start)
            && current.definition_loc.is_some()
            && current.definition_loc.as_ref() != current.usage_loc.as_ref()
        {
            self.force_definition_view = true;
            self.focused_scope = current.trace_marker_id;
            return Some(DebugNotification::CurrentEvent(current.clone()));
        }
        self.force_definition_view = false;
        self.focused_scope = None;
        if let Some(next_idx) = self.next_marker_start_after(self.cursor) {
            self.cursor = next_idx;
            return Some(DebugNotification::CurrentEvent(self.events[next_idx].clone()));
        }
        self.cursor = self.events.len();
        Some(DebugNotification::ParseComplete)
    }

    fn step_over(&mut self) -> Option<DebugNotification> {
        self.force_definition_view = false;
        let Some(current) = self.events.get(self.cursor) else {
            return Some(DebugNotification::ParseComplete);
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
            self.focused_scope = None;
            self.cursor = idx;
            return Some(DebugNotification::CurrentEvent(self.events[idx].clone()));
        }
        self.cursor = self.events.len();
        Some(DebugNotification::ParseComplete)
    }

    fn step_out(&mut self) -> Option<DebugNotification> {
        self.force_definition_view = false;
        let current_idx = self.cursor;
        let Some(_current) = self.events.get(current_idx) else {
            return Some(DebugNotification::ParseComplete);
        };
        let mut parent_start: Option<usize> = None;
        let mut parent_end: Option<usize> = None;
        for (start_idx, end_idx) in &self.marker_start_to_end {
            if *start_idx < current_idx
                && current_idx <= *end_idx
                && parent_start.is_none_or(|existing| *start_idx > existing)
            {
                parent_start = Some(*start_idx);
                parent_end = Some(*end_idx);
            }
        }
        if let Some(end_idx) = parent_end
            && let Some(next_idx) = self.next_marker_start_after(end_idx)
        {
            self.focused_scope = None;
            self.cursor = next_idx;
            return Some(DebugNotification::CurrentEvent(self.events[next_idx].clone()));
        }
        self.cursor = self.events.len();
        Some(DebugNotification::ParseComplete)
    }

    fn run_until_breakpoint_or_end(&mut self) -> Option<DebugNotification> {
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
                return Some(DebugNotification::BreakpointHit { event, reason });
            }
        }
        self.cursor = self.events.len();
        Some(DebugNotification::ParseComplete)
    }

    fn matches_breakpoint(bp: &Breakpoint, event: &NodeTrace) -> bool {
        match bp {
            Breakpoint::EventKind(kind) => event.runtime_kind.as_ref().is_some_and(|runtime_kind| {
                std::mem::discriminant(kind) == std::mem::discriminant(runtime_kind)
            }),
            Breakpoint::Label(label) => event.label.as_ref() == Some(label),
            Breakpoint::RuleId(rule_id) => event
                .rule
                .as_ref()
                .is_some_and(|rule| &rule.rule_id == rule_id),
            Breakpoint::PositionRange { start, end } => {
                event.input_start >= *start && event.input_end <= *end
            }
        }
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    pub fn total_events(&self) -> usize {
        self.events.len()
    }

    pub fn showing_definition(&self) -> bool {
        self.force_definition_view
    }

    pub fn jump_to_node_id(&mut self, node_id: u64) -> bool {
        if let Some((idx, _)) = self
            .events
            .iter()
            .enumerate()
            .find(|(_, event)| event.node_id == node_id)
        {
            self.cursor = idx;
            self.force_definition_view = false;
            self.focused_scope = None;
            return true;
        }
        false
    }

    fn next_marker_start_after(&self, idx: usize) -> Option<usize> {
        self.marker_start_indices
            .iter()
            .copied()
            .find(|marker_idx| *marker_idx > idx)
    }
}

