pub mod eval;
pub mod grammar;

use marser::error::{FurthestFailError, ParserError};

use self::eval::{RuntimeError, Value, run_file};
use self::grammar::get_mini_language_grammar;
use self::grammar::FunctionDef;

pub enum RunError {
    Parse(FurthestFailError),
    Runtime(RuntimeError),
}

pub fn parse_source<'src>(
    source: &'src str,
) -> Result<(Vec<FunctionDef<'src>>, Vec<ParserError>), FurthestFailError> {
    marser::parse(get_mini_language_grammar(), source)
}

pub fn eval_parsed<'src>(functions: &'src [FunctionDef<'src>]) -> Result<Value, RuntimeError> {
    run_file(functions)
}

pub fn run_source<'src>(source: &'src str) -> Result<(Value, Vec<ParserError>), RunError> {
    let (functions, errors) = parse_source(source).map_err(RunError::Parse)?;
    if errors.is_empty() {
        let value = run_file(&functions).map_err(RunError::Runtime)?;
        Ok((value, errors))
    } else {
        Ok((Value::Unit, errors))
    }
}
