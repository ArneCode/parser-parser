//! TUI and replay helpers for marser trace logs.
//!
//! **Experimental:** UI and replay APIs may change between releases; see the crate
//! README on GitHub for the current stability note.

mod app;
mod marker_index;
pub mod outcome;
pub mod replay;
mod state;
mod ui;

use std::io;
use std::path::PathBuf;

pub use marser_trace_schema::TraceFormat;

enum ParseArgs {
    Run {
        trace_path: PathBuf,
        source_path: Option<PathBuf>,
        format: Option<TraceFormat>,
    },
    Help,
}

/// Run the viewer after parsing the same CLI as the `marser-trace-viewer` binary.
pub fn run_cli() -> io::Result<()> {
    let program = std::env::args()
        .next()
        .unwrap_or_else(|| "marser-trace-viewer".to_string());
    let args = std::env::args().skip(1);
    let parsed = match parse_args(args) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("{err}\n\n{}", usage(&program));
            std::process::exit(2);
        }
    };
    match parsed {
        ParseArgs::Help => {
            println!("{}", usage(&program));
            std::process::exit(0);
        }
        ParseArgs::Run {
            trace_path,
            source_path,
            format,
        } => app::run(trace_path, source_path, format),
    }
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} --trace <path> [--source <path>] [--format json|jsonl]\n\n\
         Keys: i(step-into) s(step-over) u(step-up)\n\
               backspace(previous displayed event) q(quit)"
    )
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<ParseArgs, String> {
    let mut trace = None;
    let mut source = None;
    let mut format = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(ParseArgs::Help),
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
    Ok(ParseArgs::Run {
        trace_path: trace,
        source_path: source,
        format,
    })
}
