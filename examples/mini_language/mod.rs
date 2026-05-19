pub mod eval;
pub mod grammar;

use marser::error::{FurthestFailError, ParserError};
use marser::parser::Parser;

use self::eval::{RuntimeError, Value, run_file};
use self::grammar::FunctionDef;
use self::grammar::get_mini_language_grammar;

// `RunError` is only surfaced through `run_source`; the example binary does not call `run_source`.
#[allow(dead_code)]
pub enum RunError {
    Parse(FurthestFailError),
    Runtime(RuntimeError),
}

pub fn parse_source<'src>(
    source: &'src str,
) -> Result<(Vec<FunctionDef<'src>>, Vec<ParserError>), FurthestFailError> {
    let grammar = get_mini_language_grammar();
    grammar.parse_str(source)
}

#[allow(dead_code)] // Example binary uses this; integration tests use `run_source` instead.
pub fn eval_parsed<'src>(functions: &'src [FunctionDef<'src>]) -> Result<Value, RuntimeError> {
    run_file(functions)
}

// Integration tests exercise this; the example binary uses `eval_parsed` only.
#[allow(dead_code)]
pub fn run_source(source: &str) -> Result<(Value, Vec<ParserError>), RunError> {
    let (functions, errors) = parse_source(source).map_err(RunError::Parse)?;
    if errors.is_empty() {
        let value = run_file(&functions).map_err(RunError::Runtime)?;
        Ok((value, errors))
    } else {
        Ok((Value::Unit, errors))
    }
}
