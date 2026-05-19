//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::state::ViewerState;
use crate::ui;
use marser_trace_schema::{TraceFormat, load_trace_file};

pub fn run(
    trace_path: std::path::PathBuf,
    source_path: Option<std::path::PathBuf>,
    format: Option<TraceFormat>,
) -> io::Result<()> {
    let trace = load_trace_file(trace_path, format)?;
    let source_text = if let Some(source_text) = trace.source_text() {
        Some(source_text.to_string())
    } else if let Some(path) = source_path {
        Some(std::fs::read_to_string(path)?)
    } else {
        None
    };

    let mut state = ViewerState::new(trace.events().to_vec(), source_text);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut exit_requested = false;
    while !exit_requested {
        terminal.draw(|frame| ui::render(frame, &state))?;
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
