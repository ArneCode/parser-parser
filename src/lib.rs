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

#[cfg(feature = "parser-trace")]
use crate::trace::TraceFormat;
#[cfg(feature = "parser-trace")]
use crate::trace::TraceSession;
use crate::{
    context::ParserContext,
    error::{
        FurthestFailError, MatcherRunError, ParserError, error_handler::EmptyErrorHandler,
    },
    input::{Input, InputStream},
    matcher::{
        any_token::AnyToken, commit_matcher::commit_on, negative_lookahead::negative_lookahead,
    },
    parser::{Parser, internal::ParserImpl},
};

/// Parse all of `src` with the same driver as [`Parser::parse_str`](parser::Parser::parse_str)
/// / [`Parser::parse_whole_input`](parser::Parser::parse_whole_input).
///
/// Prefer `parser.parse_str(src)` or `parser.parse_whole_input(src)`; this function remains for backward compatibility.
///
/// On success returns the parsed output and any collected [`error::ParserError`] values.
/// On hard failure returns [`error::FurthestFailError`].
pub fn parse<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
) -> Result<(Out, Vec<ParserError>), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + Clone + 'src,
{
    parse_whole_input_with_default_eof(&parser, src)
}

/// Whole-input parse with default EOF wrapper: first pass with [`ParserContext::is_in_error_recovery`]
/// false; on [`MatcherRunError::RetryRerunNeeded`], rewind, reset transient state, set recovery flag,
/// and parse once more.
///
/// Works for any [`Input`](crate::input::Input) (for example `&str` or `&[T]`).
pub(crate) fn parse_whole_input_with_default_eof<'src, Pars, Inp, Out>(
    parser: &Pars,
    input: Inp,
) -> Result<(Out, Vec<ParserError>), FurthestFailError>
where
    Pars: Parser<'src, Inp, Output = Out> + Clone + 'src,
    Inp: Input<'src> + Clone + 'src,
    Out: 'src,
{
    let mut context = ParserContext::new();
    let mut input = InputStream::new(input);
    let start_pos = input.get_pos();
    let mut error_handler = EmptyErrorHandler;
    let parser = Rc::new(parser.clone());

    let eof_wrapped = capture!(
        commit_on((), (
            bind!(parser.clone(), result),
            negative_lookahead(AnyToken),
        )) => result
    );

    context.is_in_error_recovery = false;
    let first = eof_wrapped.parse(&mut context, &mut error_handler, &mut input);
    match first {
        Ok(Some(out)) => Ok((out, context.get_errors())),
        Ok(None) => {
            // this should never happen because the commit matcher assures that either a result is
            // returned or a furthest fail is raised.
            let p: usize = input.get_pos().into();
            Err(MatcherRunError::RetryRerunNeeded.into_furthest_fail_for_parser((p, p)))
        }
        Err(MatcherRunError::FurthestFail(e)) => Err(e),
        Err(MatcherRunError::RetryRerunNeeded) => {
            input.set_pos(start_pos.clone());
            context.reset_for_global_recovery_reparse();
            context.is_in_error_recovery = true;
            match eof_wrapped.parse(&mut context, &mut error_handler, &mut input) {
                Ok(Some(out)) => Ok((out, context.get_errors())),
                Ok(None) => {
                    let p: usize = input.get_pos().into();
                    Err(MatcherRunError::RetryRerunNeeded.into_furthest_fail_for_parser((p, p)))
                }
                Err(MatcherRunError::FurthestFail(e)) => Err(e),
                Err(MatcherRunError::RetryRerunNeeded) => {
                    let p: usize = input.get_pos().into();
                    Err(MatcherRunError::RetryRerunNeeded.into_furthest_fail_for_parser((p, p)))
                }
            }
        }
    }
}

#[cfg(feature = "parser-trace")]
pub(crate) fn parse_whole_input_inner_with_trace<'src, Pars, Inp, Out>(
    parser: &Pars,
    input: Inp,
    trace_session: TraceSession,
) -> (
    Result<(Out, Vec<ParserError>), FurthestFailError>,
    TraceSession,
)
where
    Pars: Parser<'src, Inp, Output = Out> + Clone + 'src,
    Inp: Input<'src> + Clone + 'src,
    Out: 'src,
{
    let mut context = ParserContext::new();
    context.attach_trace_session(trace_session);
    let mut input = InputStream::new(input);
    let start_pos = input.get_pos();
    let mut error_handler = EmptyErrorHandler;
    let parser = Rc::new(parser.clone());

    let eof_wrapped = capture!(
        commit_on((), (
            bind!(parser.clone(), result),
            negative_lookahead(AnyToken),
        )) => result
    );

    context.is_in_error_recovery = false;
    let mut parse_result = eof_wrapped.parse(&mut context, &mut error_handler, &mut input);
    if matches!(parse_result, Err(MatcherRunError::RetryRerunNeeded)) {
        input.set_pos(start_pos.clone());
        context.reset_for_global_recovery_reparse();
        context.is_in_error_recovery = true;
        parse_result = eof_wrapped.parse(&mut context, &mut error_handler, &mut input);
    }
    let trace = context.take_trace_session().unwrap_or_default();
    match parse_result {
        Ok(Some(out)) => (Ok((out, context.get_errors())), trace),
        Ok(None) => {
            let p: usize = input.get_pos().into();
            (
                Err(MatcherRunError::RetryRerunNeeded.into_furthest_fail_for_parser((p, p))),
                trace,
            )
        }
        Err(MatcherRunError::FurthestFail(e)) => (Err(e), trace),
        Err(MatcherRunError::RetryRerunNeeded) => {
            let p: usize = input.get_pos().into();
            (
                Err(MatcherRunError::RetryRerunNeeded.into_furthest_fail_for_parser((p, p))),
                trace,
            )
        }
    }
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
) -> Result<(Out, Vec<ParserError>, TraceSession), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + Clone + 'src,
{
    let (result, trace) = crate::parse_whole_input_inner_with_trace(&parser, src, TraceSession::new());
    result.map(|(output, errors)| (output, errors, trace))
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace_session<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
    trace_session: TraceSession,
) -> Result<(Out, Vec<ParserError>, TraceSession), FurthestFailError>
where
    Pars: Parser<'src, &'src str, Output = Out> + Clone + 'src,
{
    let (result, trace) =
        crate::parse_whole_input_inner_with_trace(&parser, src, trace_session);
    result.map(|(output, errors)| (output, errors, trace))
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
pub(crate) fn write_trace_to_file(
    trace: &TraceSession,
    trace_path: impl AsRef<Path>,
    format: TraceFormat,
) -> Result<(), io::Error> {
    let file = File::create(trace_path)?;
    match format {
        TraceFormat::Json => trace.write_json(file)?,
        TraceFormat::Jsonl => trace.write_jsonl(file)?,
    }
    Ok(())
}

#[cfg(feature = "parser-trace")]
pub fn parse_with_trace_to_file<'src, Pars, Out: 'src>(
    parser: Pars,
    src: &'src str,
    trace_path: impl AsRef<Path>,
    format: TraceFormat,
) -> Result<(Out, Vec<ParserError>), ParseWithTraceToFileError>
where
    Pars: Parser<'src, &'src str, Output = Out> + Clone + 'src,
{
    let (result, mut trace) =
        crate::parse_whole_input_inner_with_trace(&parser, src, TraceSession::new());
    trace.set_source_text(src);
    crate::write_trace_to_file(&trace, trace_path, format)?;
    match result {
        Ok((output, errors)) => Ok((output, errors)),
        Err(parse_err) => Err(ParseWithTraceToFileError::Parse(parse_err)),
    }
}
