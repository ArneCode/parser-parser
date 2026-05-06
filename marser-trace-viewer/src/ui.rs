use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::state::ViewerState;

pub fn render(frame: &mut Frame<'_>, state: &ViewerState) {
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
