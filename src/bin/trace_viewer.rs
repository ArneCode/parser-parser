use std::{collections::HashMap, env, fs, io, path::PathBuf, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use marser::trace::{NodeTrace, TraceMarkerPhase, load::{TraceFormat, load_trace_file}};
use ratatui::{
    Frame, Terminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Clone, Copy)]
enum MarkerOutcome {
    Running,
    Success,
    /// Matched, but `recover_with` succeeded inside this marker span (see `RecoverSuccess` events).
    Recovered,
    Fail,
    Error,
    Backtrack,
}

impl MarkerOutcome {
    /// Step / current marker outcome: only `fail`, `success`, `recovered error`, or `error`.
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Recovered => "recovered error",
            Self::Fail | Self::Running | Self::Backtrack => "fail",
            Self::Error => "error",
        }
    }
}

#[derive(Clone)]
struct ViewerState {
    events: Vec<NodeTrace>,
    start_indices: Vec<usize>,
    start_to_end: HashMap<usize, usize>,
    current_idx: Option<usize>,
    history: Vec<usize>,
    history_cursor: Option<usize>,
    source_text: Option<String>,
    grammar_sources: HashMap<String, String>,
    last_step_result: String,
}

impl ViewerState {
    fn new(events: Vec<NodeTrace>, source_text: Option<String>) -> Self {
        let start_indices = events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_explicit_trace_marker && matches!(e.marker_phase, TraceMarkerPhase::Start))
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

        let mut state = Self {
            events,
            start_indices,
            start_to_end,
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

    fn current_event(&self) -> Option<&NodeTrace> {
        self.current_idx.and_then(|idx| self.events.get(idx))
    }

    /// Byte range in the input covered by the current explicit trace marker (paired start→end).
    /// Half-open `[lo, hi)` with at least one byte when `lo < len`, matching the blue “point” rule.
    fn marker_pair_extent_range(&self, source_len: usize) -> Option<(usize, usize)> {
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
        let first_user = self.start_indices.iter().copied().find(|idx| {
            self.events[*idx]
                .usage_loc
                .as_ref()
                .is_some_and(|loc| !loc.file.ends_with("/src/lib.rs"))
        }).or_else(|| self.start_indices.first().copied());
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

    fn ensure_grammar_loaded(&mut self) {
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

    fn set_current_index(&mut self, idx: usize) {
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

    fn step_into(&mut self) {
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

    fn step_over(&mut self) {
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

    fn step_out(&mut self) {
        let Some(current) = self.current_idx else {
            self.last_step_result = "u: no current marker".to_string();
            return;
        };
        let mut parent_end: Option<usize> = None;
        for (start, end) in &self.start_to_end {
            if *start < current && current <= *end && parent_end.is_none_or(|existing| *end < existing) {
                parent_end = Some(*end);
            }
        }
        if let Some(end_idx) = parent_end {
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

    fn go_back(&mut self) {
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
        self.events
            .get(idx)
            .and_then(|event| event.label.as_deref().or(event.rule.as_ref().and_then(|r| r.rule_name.as_deref())))
    }

    fn current_label(&self) -> Option<&str> {
        self.current_idx.and_then(|idx| self.label_at(idx))
    }

    fn current_outcome(&self) -> MarkerOutcome {
        self.current_idx
            .map(|idx| self.outcome_for_start(idx))
            .unwrap_or(MarkerOutcome::Running)
    }

    fn outcome_for_start(&self, start_idx: usize) -> MarkerOutcome {
        if let Some(end_idx) = self.start_to_end.get(&start_idx).copied() {
            let start = &self.events[start_idx];
            // Between Start and paired End there are nested runtime events (many one_of failures
            // etc.); those are not outcomes of THIS trace marker — only propagate hard errors here.
            let mut saw_error = false;
            for event in &self.events[(start_idx + 1)..end_idx] {
                if matches!(
                    event.runtime_kind,
                    Some(marser::trace::TraceEventKind::MatchHardError)
                ) {
                    saw_error = true;
                    break;
                }
            }
            if saw_error {
                return MarkerOutcome::Error;
            }
            let mut saw_recovery = false;
            let mut max_error_stack_len = start.error_stack_len;
            for event in &self.events[(start_idx + 1)..end_idx] {
                if matches!(
                    event.runtime_kind,
                    Some(marser::trace::TraceEventKind::RecoverSuccess)
                ) {
                    saw_recovery = true;
                }
                max_error_stack_len = max_error_stack_len.max(event.error_stack_len);
            }
            let end = &self.events[end_idx];
            max_error_stack_len = max_error_stack_len.max(end.error_stack_len);
            if matches!(
                end.runtime_kind,
                Some(marser::trace::TraceEventKind::MatchHardError)
            ) {
                return MarkerOutcome::Error;
            }
            let recovered_by_counters = max_error_stack_len > start.error_stack_len;
            if saw_recovery || recovered_by_counters {
                return MarkerOutcome::Recovered;
            }
            return match end.status {
                marser::trace::NodeTraceStatus::Success => MarkerOutcome::Success,
                marser::trace::NodeTraceStatus::Fail => MarkerOutcome::Fail,
                marser::trace::NodeTraceStatus::Backtrack => MarkerOutcome::Backtrack,
                marser::trace::NodeTraceStatus::Enter => MarkerOutcome::Running,
            };
        }

        // Unpaired start (truncated trace / legacy): heuristic only.
        let window_end_exclusive = self
            .next_start_after(start_idx)
            .unwrap_or_else(|| self.events.len());
        let mut saw_error = false;
        let mut saw_fail = false;
        let mut saw_success_end = false;
        let mut saw_backtrack = false;
        let mut saw_recovery = false;
        let start = &self.events[start_idx];
        let mut max_error_stack_len = start.error_stack_len;
        for event in &self.events[start_idx..window_end_exclusive] {
            if matches!(
                event.runtime_kind,
                Some(marser::trace::TraceEventKind::MatchHardError)
            ) {
                saw_error = true;
            }
            if matches!(
                event.runtime_kind,
                Some(marser::trace::TraceEventKind::RecoverSuccess)
            ) {
                saw_recovery = true;
            }
            max_error_stack_len = max_error_stack_len.max(event.error_stack_len);
            match event.status {
                marser::trace::NodeTraceStatus::Backtrack => saw_backtrack = true,
                marser::trace::NodeTraceStatus::Fail => saw_fail = true,
                marser::trace::NodeTraceStatus::Success
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
        } else if saw_recovery || max_error_stack_len > start.error_stack_len {
            MarkerOutcome::Recovered
        } else if saw_success_end {
            MarkerOutcome::Success
        } else if saw_backtrack {
            MarkerOutcome::Backtrack
        } else {
            MarkerOutcome::Running
        }
    }

    fn marker_context_text(&self) -> Text<'static> {
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
                            "outcome: recovered error — RecoverSuccess inside this marker span",
                        ),
                        Line::from(format!(
                            "consumed: {n} bytes (input {}..{})",
                            start_ev.input_start, end.input_end
                        )),
                    ]
                } else {
                    vec![Line::from(
                        "outcome: recovered error — no paired end; look for RecoverSuccess in timeline",
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

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} --trace <path> [--source <path>] [--format json|jsonl]\n\n\
         Keys: i(step-into) s(step-over) u(step-up)\n\
               backspace(previous displayed event) q(quit)"
    )
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>, Option<TraceFormat>), String> {
    let mut args = env::args().skip(1);
    if let Some(first) = args.next() {
        if first == "--help" || first == "-h" {
            return Err("__PRINT_HELP__".to_string());
        }
        args = env::args().skip(1);
    }
    let mut trace = None;
    let mut source = None;
    let mut format = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = args.next().map(PathBuf::from),
            "--source" => source = args.next().map(PathBuf::from),
            "--format" => {
                format = match args.next().as_deref() {
                    Some("json") => Some(TraceFormat::Json),
                    Some("jsonl") => Some(TraceFormat::Jsonl),
                    _ => return Err("Invalid format. Use json or jsonl".to_string()),
                }
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    let trace = trace.ok_or_else(|| "Missing required --trace <path>".to_string())?;
    Ok((trace, source, format))
}

fn main() -> io::Result<()> {
    let program = env::args()
        .next()
        .unwrap_or_else(|| "trace_viewer".to_string());
    let (trace_path, source_path, format) = match parse_args() {
        Ok(v) => v,
        Err(err) => {
            if err == "__PRINT_HELP__" {
                println!("{}", usage(&program));
                std::process::exit(0);
            }
            eprintln!("{err}\n\n{}", usage(&program));
            std::process::exit(2);
        }
    };
    let trace = load_trace_file(trace_path, format)?;
    let source_text = if let Some(path) = source_path {
        Some(fs::read_to_string(path)?)
    } else {
        None
    };

    let mut state = ViewerState::new(trace.events().to_vec(), source_text);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut exit_requested = false;
    while !exit_requested {
        terminal.draw(|frame| render(frame, &state))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => exit_requested = true,
                KeyCode::Char('i') => state.step_into(),
                KeyCode::Char('s') => state.step_over(),
                KeyCode::Char('u') => state.step_out(),
                KeyCode::Backspace => state.go_back(),
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render(frame: &mut Frame<'_>, state: &ViewerState) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(11)])
        .split(frame.area());
    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(root[1]);
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[0]);

    let grammar_preview = render_grammar_preview(state);
    frame.render_widget(
        Paragraph::new(grammar_preview)
            .block(Block::default().title("Grammar Source").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        outer[0],
    );

    let source_preview = render_source_preview(state);
    let event_header = if let Some(event) = state.current_event() {
        format!(
            "Source Input / event#{} {:?} span {}..{}",
            event.node_id, event.runtime_kind, event.input_start, event.input_end
        )
    } else {
        "Source Input".to_string()
    };
    let mut source_lines = vec![
        Line::from("controls: i s u backspace q"),
        Line::from(""),
    ];
    source_lines.extend(source_preview.lines);
    frame.render_widget(
        Paragraph::new(Text::from(source_lines))
            .block(
                Block::default()
                    .title(event_header)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: false }),
        outer[1],
    );

    let current_pos = state
        .current_idx
        .and_then(|idx| state.start_indices.iter().position(|start| *start == idx));
    let total = state.start_indices.len();
    let trace_status = if let Some(event) = state.current_event() {
        format!(
            "trace: cursor {}/{} | shown id={} | kind={:?} | outcome={} | span={}..{}",
            current_pos.unwrap_or(0).saturating_add(1),
            total.max(1),
            event.node_id,
            event.runtime_kind,
            state.current_outcome().as_str(),
            event.input_start,
            event.input_end
        )
    } else {
        format!("trace: cursor -/{} | no current shown event", total.max(1))
    };
    frame.render_widget(
        Paragraph::new(trace_status).block(Block::default().title("Trace Position").borders(Borders::ALL)),
        bottom[0],
    );
    frame.render_widget(
        Paragraph::new(state.last_step_result.as_str())
            .block(Block::default().borders(Borders::ALL).title("Step / current")),
        bottom[1],
    );
    frame.render_widget(
        Paragraph::new(state.marker_context_text())
            .block(Block::default().borders(Borders::ALL).title("Marker context"))
            .wrap(Wrap { trim: true }),
        bottom[2],
    );
}

fn render_grammar_preview(state: &ViewerState) -> Text<'static> {
    let highlight_style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED);
    let Some(event) = state.current_event() else {
        return Text::from("No current event.");
    };
    let Some(rule) = event.rule.as_ref() else {
        return Text::from("Current event has no usage metadata.");
    };
    let Some(grammar_source) = state.grammar_sources.get(&rule.rule_file) else {
        return Text::from(format!("Could not load grammar file '{}'.", rule.rule_file));
    };
    let lines: Vec<&str> = grammar_source.lines().collect();
    if lines.is_empty() {
        return Text::from("Grammar file is empty.");
    }
    let active_loc = event.usage_loc.as_ref().or(event.definition_loc.as_ref());
    let Some(active_loc) = active_loc else {
        return Text::from("Current event has no location.");
    };
    let line_idx = (active_loc.line as usize).saturating_sub(1).min(lines.len() - 1);
    let from = line_idx.saturating_sub(10);
    let to = (line_idx + 11).min(lines.len());
    let rule_line = lines[line_idx];
    let line_len = rule_line.len();
    let mut hl_start = (active_loc.column as usize).saturating_sub(1).min(line_len);
    let mut hl_end = hl_start;
    if let Some(name) = rule.rule_name.as_deref() {
        if !name.is_empty() {
            let tentative_end = (hl_start + name.len()).min(line_len);
            if tentative_end > hl_start && rule_line[hl_start..tentative_end] == *name {
                hl_end = tentative_end;
            }
        }
    }
    if hl_end <= hl_start && hl_start < line_len {
        let bytes = rule_line.as_bytes();
        while hl_start > 0
            && (bytes[hl_start - 1].is_ascii_alphanumeric() || bytes[hl_start - 1] == b'_')
        {
            hl_start -= 1;
        }
        hl_end = hl_start;
        while hl_end < line_len
            && (bytes[hl_end].is_ascii_alphanumeric() || bytes[hl_end] == b'_')
        {
            hl_end += 1;
        }
        if hl_end <= hl_start {
            hl_end = (hl_start + 1).min(line_len);
        }
    }
    let mut out_lines = vec![
        Line::from(format!(
        "rule_id={} name={} location={}:{}:{}\n\n",
        rule.rule_id,
        rule.rule_name.as_deref().unwrap_or("-"),
        active_loc.file,
        active_loc.line,
        active_loc.column
        )),
        Line::from(""),
    ];
    for (idx, line) in lines[from..to].iter().enumerate() {
        let real_idx = from + idx;
        let prefix = format!("{:4} | ", real_idx + 1);
        if real_idx == line_idx {
            if hl_end > hl_start {
                out_lines.push(Line::from(vec![
                    Span::raw(prefix),
                    Span::raw(line[..hl_start].to_string()),
                    Span::styled(line[hl_start..hl_end].to_string(), highlight_style),
                    Span::raw(line[hl_end..].to_string()),
                ]));
            } else {
                out_lines.push(Line::from(format!("{prefix}{line}")));
            }
        } else {
            out_lines.push(Line::from(format!("{prefix}{line}")));
        }
    }
    Text::from(out_lines)
}

fn render_source_preview(state: &ViewerState) -> Text<'static> {
    let blue_style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED);
    // Full marker span: underline only (no extra color).
    let marker_extent_style = Style::default().add_modifier(Modifier::UNDERLINED);
    if let (Some(event), Some(source)) = (state.current_event(), state.source_text.as_ref()) {
        let len = source.len();
        let blue_lo = event.input_start.min(len);
        let blue_hi_raw = event.input_end.min(len);
        let blue_hi = if blue_hi_raw > blue_lo {
            blue_hi_raw
        } else {
            (blue_lo + 1).min(len)
        };

        let green_range = state.marker_pair_extent_range(len);

        let mut line_starts = vec![0usize];
        for (idx, ch) in source.char_indices() {
            if ch == '\n' && idx + 1 <= source.len() {
                line_starts.push(idx + 1);
            }
        }
        let find_line = |byte_idx: usize| -> usize {
            match line_starts.binary_search(&byte_idx) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            }
        };

        let mut start_line = find_line(blue_lo);
        let mut end_line = find_line(blue_hi.saturating_sub(1));
        if let Some((g_lo, g_hi)) = green_range {
            start_line = start_line.min(find_line(g_lo));
            end_line = end_line.max(find_line(g_hi.saturating_sub(1)));
        }
        let from_line = start_line.saturating_sub(8);
        let to_line = (end_line + 9).min(line_starts.len());

        let overlaps = |a0: usize, a1: usize, b0: usize, b1: usize| -> bool {
            a1 > b0 && a0 < b1
        };

        let mut lines = Vec::new();
        for line_idx in from_line..to_line {
            let line_start = line_starts[line_idx];
            let line_end = if line_idx + 1 < line_starts.len() {
                line_starts[line_idx + 1].saturating_sub(1)
            } else {
                source.len()
            };
            let line_text = &source[line_start..line_end];
            let line_len = line_text.len();
            let line_abs_end = line_start + line_len;

            let mut breaks: Vec<usize> = vec![0, line_len];
            let mut push_interval = |g0: usize, g1: usize| {
                let a = g0.max(line_start).min(line_abs_end);
                let b = g1.max(line_start).min(line_abs_end);
                if a < b {
                    let la = a.saturating_sub(line_start);
                    let lb = b.saturating_sub(line_start);
                    breaks.push(la.min(line_len));
                    breaks.push(lb.min(line_len));
                }
            };
            push_interval(blue_lo, blue_hi);
            if let Some((g_lo, g_hi)) = green_range {
                push_interval(g_lo, g_hi);
            }
            breaks.sort_unstable();
            breaks.dedup();

            let prefix = format!("{:4} | ", line_idx + 1);
            let mut spans: Vec<Span<'static>> = vec![Span::raw(prefix)];
            let mut bi = 0usize;
            while bi + 1 < breaks.len() {
                let b0 = breaks[bi];
                let b1 = breaks[bi + 1];
                if b0 < b1 {
                    let abs0 = line_start + b0;
                    let abs1 = line_start + b1;
                    let piece = line_text[b0..b1].to_string();
                    if overlaps(abs0, abs1, blue_lo, blue_hi) {
                        spans.push(Span::styled(piece, blue_style));
                    } else if green_range.is_some_and(|(g0, g1)| overlaps(abs0, abs1, g0, g1)) {
                        spans.push(Span::styled(piece, marker_extent_style));
                    } else {
                        spans.push(Span::raw(piece));
                    }
                }
                bi += 1;
            }
            lines.push(Line::from(spans));
        }

        Text::from(lines)
    } else if let Some(event) = state.current_event() {
        Text::from(format!(
            "No source loaded. Current span: {}..{}",
            event.input_start, event.input_end
        ))
    } else {
        Text::from("No source loaded.")
    }
}

