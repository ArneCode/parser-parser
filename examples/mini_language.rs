use std::{env, fs, process};

use marser::error::ParserError;

#[path = "mini_language/mod.rs"]
mod mini_language_app;

fn main() {
    let mut args = env::args();
    let program_name = args.next().unwrap_or_else(|| "mini_language".to_string());
    let Some(path) = args.next() else {
        eprintln!("usage: {program_name} <script.ml>");
        process::exit(2);
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("failed to read '{path}': {err}");
            process::exit(1);
        }
    };

    match mini_language_app::parse_source(&source) {
        Ok((functions, errors)) => {
            if !errors.is_empty() {
                eprintln!("recovered with {} diagnostic(s):", errors.len());
                ParserError::eprint_many(&errors, &path, &source);
                println!("Recovered AST:\n{functions:#?}");
                process::exit(1);
            }

            let value = match mini_language_app::eval_parsed(&functions) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("{err}");
                    process::exit(1);
                }
            };
            println!("{value}");
        }
        Err(err) => {
            err.eprint(&path, &source);
            process::exit(1);
        }
    };
}
