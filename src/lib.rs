#![doc = include_str!("../README.md")]
// `Parser` and `Matcher` are sealed via `pub(crate)` supertraits (`ParserImpl`,
// `MatcherImpl`) whose method signatures intentionally reference crate-private
// runtime types (`ParserContext`, `ErrorHandler`, `MatchRunner`). The
// `private_bounds` lint flags every `impl …Impl for …` site in the crate (~130
// locations) for this single intentional pattern. Downstream users cannot name
// or implement `ParserImpl`/`MatcherImpl`, so the "reachability" the lint warns
// about is purely nominal.
#![allow(private_bounds, private_interfaces)]
extern crate self as marser;

pub(crate) mod context;
pub mod error;
pub mod guide;
pub mod input;
pub mod label;
pub mod matcher;
pub mod one_of;
pub mod parser;
pub mod trace;

use marser_macros::capture;
use std::rc::Rc;
#[cfg(feature = "parser-trace")]
use std::{fs::File, io, path::Path};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, ParserError, error_handler::EmptyErrorHandler},
    input::InputStream,
    matcher::{
        any_token::AnyToken, commit_matcher::commit_on, negative_lookahead::negative_lookahead,
    },
    parser::{Parser, internal::ParserImpl},
};
#[cfg(feature = "parser-trace")]
use crate::trace::TraceSession;
#[cfg(feature = "parser-trace")]
use crate::trace::load::TraceFormat;

/// Parse all of `src` with a small driver around `parser`.
///
/// - Runs [`Parser::parse`](parser::Parser) against an [`input::InputStream`] over `src`.
/// - Wraps your parser in `capture!(commit_on((), (bind!(…), negative_lookahead(AnyToken))) => …)`
///   so the whole input must be consumed (commit, bind result, then forbid trailing tokens).
///
/// On success returns the parsed output and any collected [`error::ParserError`] values.
/// On hard failure returns [`error::FurthestFailError`]. For a custom token type or context,
/// call `parse` on your [`parser::Parser`] implementation directly instead.
pub fn parse<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
) -> Result<(Out, Vec<ParserError>), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + 'src,
{
    let mut error_handler = EmptyErrorHandler;
    let mut context = ParserContext::new();
    let mut input = InputStream::new(src);
    let parser = Rc::new(parser);

    let parser = capture!(
        commit_on((), (
            bind!(parser.clone(), result),
            negative_lookahead(AnyToken),
        )) => result
    );
    let result = parser
        .parse(&mut context, &mut error_handler, &mut input)?
        .unwrap();
    Ok((result, context.get_errors()))
}

#[cfg(feature = "parser-trace")]
fn parse_inner_with_trace<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
    trace_session: TraceSession,
) -> Result<(Out, Vec<ParserError>, TraceSession), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + 'src,
{
    let mut error_handler = EmptyErrorHandler;
    let mut context = ParserContext::new();
    context.attach_trace_session(trace_session);
    let mut input = InputStream::new(src);
    let parser = Rc::new(parser);

    let parser = capture!(
        commit_on((), (
            bind!(parser.clone(), result),
            negative_lookahead(AnyToken),
        )) => result
    );
    let result = parser
        .parse(&mut context, &mut error_handler, &mut input)?
        .unwrap();
    let trace = context.take_trace_session().unwrap_or_default();
    let errors = context.get_errors();
    Ok((
        result,
        errors,
        trace,
    ))
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
) -> Result<(Out, Vec<ParserError>, TraceSession), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + 'src,
{
    parse_inner_with_trace(parser, src, TraceSession::new())
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace_session<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
    trace_session: TraceSession,
) -> Result<(Out, Vec<ParserError>, TraceSession), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + 'src,
{
    parse_inner_with_trace(parser, src, trace_session)
}

#[cfg(feature = "parser-trace")]
#[derive(Debug)]
pub enum ParseWithTraceToFileError {
    Parse(FurthestFailError),
    Io(io::Error),
}

#[cfg(feature = "parser-trace")]
impl From<FurthestFailError> for ParseWithTraceToFileError {
    fn from(value: FurthestFailError) -> Self {
        Self::Parse(value)
    }
}

#[cfg(feature = "parser-trace")]
impl From<io::Error> for ParseWithTraceToFileError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace_to_file<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
    trace_path: impl AsRef<Path>,
    format: TraceFormat,
) -> Result<(Out, Vec<ParserError>), ParseWithTraceToFileError>
where
    Pars: Parser<'src, &'src str, Output = Out> + 'src,
{
    let (output, errors, trace) = parse_with_trace(parser, src)?;
    let file = File::create(trace_path)?;
    match format {
        TraceFormat::Json => trace.write_json(file)?,
        TraceFormat::Jsonl => trace.write_jsonl(file)?,
    }
    Ok((output, errors))
}
