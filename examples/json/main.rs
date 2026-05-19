mod grammar;

#[cfg(not(test))]
use std::{env, fs, process};

#[cfg(not(test))]
use marser::error::ParserError;
use marser::parser::Parser;
#[cfg(all(feature = "parser-trace", not(test)))]
use marser::trace::TraceFormat;

use grammar::{JsonValue, get_json_grammar};

#[cfg(not(test))]
const DEFAULT_JSON_PATH: &str = "tests/data/json1.json";

#[cfg(not(test))]
fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {program} <path-to-json-file>{}",
        if cfg!(feature = "parser-trace") {
            " [--trace-file <path>]"
        } else {
            ""
        }
    );
    process::exit(2);
}

#[cfg(not(test))]
fn print_parse_ok(value: &JsonValue<'_>, errors: &[ParserError], path: &str, source: &str) {
    ParserError::eprint_many(errors, path, source);
    println!("\n--- Recovered JSON: ---");
    println!("{}", value.serialize_pretty());
}

#[cfg(not(test))]
struct Cli {
    path: String,
    /// When set, write trace JSON here (`parser-trace` only).
    #[cfg(feature = "parser-trace")]
    trace_file: Option<String>,
}

#[cfg(not(test))]
impl Cli {
    fn parse(mut args: env::Args) -> Self {
        let program = args.next().unwrap_or_else(|| "json".to_string());
        let mut path = DEFAULT_JSON_PATH.to_string();
        #[cfg(feature = "parser-trace")]
        let mut trace_file: Option<String> = None;
        #[cfg(feature = "parser-trace")]
        let mut expect_trace_path = false;

        for arg in args {
            #[cfg(feature = "parser-trace")]
            {
                if expect_trace_path {
                    trace_file = Some(arg);
                    expect_trace_path = false;
                    continue;
                }
                if arg == "--trace-file" {
                    expect_trace_path = true;
                    continue;
                }
            }

            if path != DEFAULT_JSON_PATH {
                usage(&program);
            }
            path = arg;
        }

        Self {
            path,
            #[cfg(feature = "parser-trace")]
            trace_file,
        }
    }
}

#[cfg(not(test))]
fn read_source(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read '{path}': {err}");
            process::exit(1);
        }
    }
}

#[cfg(all(not(test), feature = "parser-trace"))]
fn run_traced<'src, P>(parser: P, sample: &'src str, cli: &Cli)
where
    P: Parser<'src, &'src str, Output = JsonValue<'src>> + Clone + 'src,
{
    let path = cli.path.as_str();
    if let Some(trace_file_path) = cli.trace_file.as_ref() {
        match parser.parse_str_with_trace_to_file(sample, trace_file_path, TraceFormat::Json) {
            Ok((value, errors)) => {
                eprintln!("trace written to {trace_file_path}");
                print_parse_ok(&value, &errors, path, sample);
            }
            Err(marser::ParseWithTraceToFileError::Parse(err)) => err.eprint(path, sample),
            Err(marser::ParseWithTraceToFileError::Io(err)) => {
                eprintln!("Failed to write trace file '{trace_file_path}': {err}");
                process::exit(1);
            }
        }
    } else {
        match parser.parse_str_with_trace(sample) {
            Ok((value, errors, _trace)) => {
                print_parse_ok(&value, &errors, path, sample);
            }
            Err(err) => err.eprint(path, sample),
        }
    }
}

#[cfg(all(not(test), not(feature = "parser-trace")))]
fn run_plain<'src, P>(parser: P, sample: &'src str, path: &str)
where
    P: Parser<'src, &'src str, Output = JsonValue<'src>> + Clone + 'src,
{
    match parser.parse_str(sample) {
        Ok((value, errors)) => {
            print_parse_ok(&value, &errors, path, sample);
        }
        Err(err) => err.eprint(path, sample),
    }
}

#[cfg(not(test))]
fn main() {
    let cli = Cli::parse(env::args());
    let sample = read_source(&cli.path);
    let parser = get_json_grammar();
    #[cfg(feature = "parser-trace")]
    run_traced(parser, sample.as_str(), &cli);
    #[cfg(not(feature = "parser-trace"))]
    run_plain(parser, sample.as_str(), cli.path.as_str());
}
