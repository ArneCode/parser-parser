//! Parsers: typed values produced from input.
//!
//! # For users
//!
//! - Build parsers with [`crate::capture`], [`crate::one_of::one_of`], [`deferred::recursive`], and
//!   the concrete parser helpers in this module (`token_parser`, ranges, [`Capture`], …).
//! - Run them with [`Parser::parse_str`] / [`Parser::parse_whole_input`] (same whole-input + EOF
//!   wrapper as [`crate::parse`]).
//! - Extend them with [`ParserCombinator`]: [`ParserCombinator::recover_with`],
//!   [`ParserCombinator::memoized`], [`ParserCombinator::map_output`], [`ParserCombinator::erase_types`], …
//!
//! Concept guides: [`crate::guide::quickstart`], [`crate::guide::capture_and_binds`],
//! [`crate::guide::errors_and_recovery`], [`crate::guide::common_patterns`].
//!
//! # Sealed implementation
//!
//! [`Parser`] is not implemented outside this crate: it extends a crate-private supertrait so only
//! types defined here satisfy the full bound.
//!
//! # Runtime invariants
//!
//! - The crate-private `ParserImpl::parse` entry point receives the parse context type
//!   `ParserContext` (not re-exported at the crate root) and must not retain references to it past
//!   the call unless owned data is explicitly `'src`-bounded (for example memoized [`Rc`] outputs).
//! - [`capture::Capture`] is normally constructed via [`crate::capture`]; bind slot layout matches
//!   [`capture::MatchResult`] tuple indexing described by the [`capture::Property`] helpers.
//!
//! ## Associated constants
//!
//! Implementations expose `CAN_FAIL`. When `true`, the parser may return `Ok(None)` on a normal path
//! (no match at the current position). It does **not** describe whether `Err` with
//! [`crate::error::MatcherRunError`] is possible on the crate-private `ParserImpl::parse` path.

pub mod capture;
pub mod deferred;
/// Type-erased parser wrapper and helpers.
pub mod erase_types;
/// Opaque `impl Parser` wrapper ([`impl_parser::as_parser`]); kept separate from [`capture`].
pub mod impl_parser;
pub mod memoized;
pub mod multiple;
/// Output-mapping parser wrapper.
pub mod output_mapper;
pub mod range_parser;
pub mod recover_error;
pub mod single_token;
pub mod token_parser;

pub use capture::{
    BindDebugInfo, BoundResult, BoundValue, Capture, MultipleProperty, OptionalProperty, Property,
    ResultBinder, SingleProperty, SpanBinder, bind_result, bind_span,
};
pub use deferred::{Deferred, DeferredWeak, recursive};
pub use impl_parser::as_parser;
pub use memoized::Memoized;
pub use multiple::MultipleParser;
pub use range_parser::RangeParser;
pub use recover_error::ErrorRecoverer;
pub use single_token::SingleTokenParser;
use std::rc::Rc;
pub use token_parser::{TokenParser, token_parser};

#[cfg(feature = "parser-trace")]
use crate::trace::{TraceFormat, TraceSession};
use crate::{
    context::ParserContext,
    error::{
        FurthestFailError, MatcherRunError, ParserError,
        error_handler::{ErrorHandler, ErrorHandlerChoice},
    },
    input::{Input, InputStream},
    matcher::{ErrorContextualizer, ignore_result::IgnoreResult},
    parser::recover_error::ErrorRecoverer as ErrorRecovererInner,
};

pub(crate) mod internal {
    use std::fmt::{Debug, Display};

    use crate::{
        context::ParserContext,
        error::{MatcherRunError, error_handler::ErrorHandler},
        input::{Input, InputStream},
        parser::ParserCombinator,
    };

    /// Crate-private parsing interface used by [`super::Parser`].
    pub trait ParserImpl<'src, Inp>: Debug + ParserCombinator + Clone
    where
        Inp: Input<'src>,
    {
        /// Successful parse value when the parser matches at `pos`.
        type Output;
        /// `true` when this parser can return `Ok(None)` on a normal parse path.
        ///
        /// This constant models parse absence and does not indicate whether
        /// `Err(MatcherRunError)` may be returned.
        const CAN_FAIL: bool;

        /// Run the parser at `pos` against `context`, reporting secondary issues through `error_handler`.
        fn parse(
            &self,
            context: &mut ParserContext<'src>,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, Inp>,
        ) -> Result<Option<Self::Output>, MatcherRunError>;

        #[inline]
        fn maybe_label(&self) -> Option<Box<dyn Display>> {
            None
        }
    }
}

/// Object-safe facade for parsers over a token type `Token`.
///
/// Typical outcomes when used through [`Self::parse_str`] / [`Self::parse_whole_input`]:
///
/// - **`Ok((output, errors))`**: parse succeeded; `errors` may still list recovered diagnostics.
/// - **`Err(FurthestFailError)`**: hard failure (often after a committed [`crate::matcher::commit_on`] rule).
///
/// For the full-input driver and recovery semantics, see [`crate::guide::errors_and_recovery`].
///
/// Blanket-implemented for every type that implements the crate-private parsing
/// trait used internally. Use [`ParserCombinator::recover_with`] and
/// [`ParserCombinator::memoized`] for common extensions; the three-argument
/// `ParserImpl::parse` method drives the actual parse step.
///
/// For parsing a full buffer with the same end-of-input wrapper as [`crate::parse`], use
/// [`Self::parse_whole_input`] (or [`Self::parse_str`] when `Inp = &'src str`).
///
/// With the `parser-trace` feature, traced runs use [`Self::parse_whole_input_with_trace`]
/// (or [`Self::parse_str_with_trace`] for `&str`).
pub trait Parser<'src, Inp: Input<'src>>: internal::ParserImpl<'src, Inp>
where
    Self: 'src,
{
    /// Parse all of `input` with the default whole-input + EOF [`crate::matcher::commit_matcher::commit_on`]
    /// wrapper (same driver as [`crate::parse`] for `Inp = &'src str`).
    fn parse_whole_input(
        &self,
        input: Inp,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, Inp>>::Output,
            Vec<ParserError>,
        ),
        FurthestFailError,
    >
    where
        Self: Clone,
        Inp: Clone + 'src,
    {
        crate::parse_whole_input_with_default_eof(self, input)
    }

    /// Like [`Self::parse_whole_input`] for string input (convenience alias).
    fn parse_str(
        &self,
        src: &'src str,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, &'src str>>::Output,
            Vec<ParserError>,
        ),
        FurthestFailError,
    >
    where
        Self: Parser<'src, &'src str> + Clone,
    {
        crate::parse_whole_input_with_default_eof(self, src)
    }

    /// Parse with tracing; same whole-input + EOF wrapper as [`Self::parse_whole_input`].
    #[cfg(feature = "parser-trace")]
    fn parse_whole_input_with_trace(
        &self,
        input: Inp,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, Inp>>::Output,
            Vec<ParserError>,
            TraceSession,
        ),
        FurthestFailError,
    >
    where
        Self: Clone,
        Inp: Clone + 'src,
    {
        let (result, trace) =
            crate::parse_whole_input_inner_with_trace(self, input, TraceSession::new());
        result.map(|(output, errors)| (output, errors, trace))
    }

    /// Like [`Self::parse_whole_input_with_trace`], reusing an existing [`TraceSession`].
    #[cfg(feature = "parser-trace")]
    fn parse_whole_input_with_trace_session(
        &self,
        input: Inp,
        trace_session: TraceSession,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, Inp>>::Output,
            Vec<ParserError>,
            TraceSession,
        ),
        FurthestFailError,
    >
    where
        Self: Clone,
        Inp: Clone + 'src,
    {
        let (result, trace) = crate::parse_whole_input_inner_with_trace(self, input, trace_session);
        result.map(|(output, errors)| (output, errors, trace))
    }

    /// Convenience alias for [`Self::parse_whole_input_with_trace`] on string input.
    #[cfg(feature = "parser-trace")]
    fn parse_str_with_trace(
        &self,
        src: &'src str,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, &'src str>>::Output,
            Vec<ParserError>,
            TraceSession,
        ),
        FurthestFailError,
    >
    where
        Self: Parser<'src, &'src str> + Clone,
    {
        self.parse_whole_input_with_trace(src)
    }

    /// Traced parse and write the session to `trace_path` (no source snapshot; spans are positions in `input`).
    #[cfg(feature = "parser-trace")]
    fn parse_whole_input_with_trace_to_file(
        &self,
        input: Inp,
        trace_path: impl AsRef<std::path::Path>,
        format: TraceFormat,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, Inp>>::Output,
            Vec<ParserError>,
        ),
        crate::ParseWithTraceToFileError,
    >
    where
        Self: Clone,
        Inp: Clone + 'src,
    {
        let (result, trace) =
            crate::parse_whole_input_inner_with_trace(self, input, TraceSession::new());
        crate::write_trace_to_file(&trace, trace_path, format)?;
        match result {
            Ok((output, errors)) => Ok((output, errors)),
            Err(parse_err) => Err(crate::ParseWithTraceToFileError::Parse(parse_err)),
        }
    }

    /// Like [`Self::parse_whole_input_with_trace_to_file`], and records `src` as trace source text.
    #[cfg(feature = "parser-trace")]
    fn parse_str_with_trace_to_file(
        &self,
        src: &'src str,
        trace_path: impl AsRef<std::path::Path>,
        format: TraceFormat,
    ) -> Result<
        (
            <Self as internal::ParserImpl<'src, &'src str>>::Output,
            Vec<ParserError>,
        ),
        crate::ParseWithTraceToFileError,
    >
    where
        Self: Parser<'src, &'src str> + Clone,
    {
        let (result, mut trace) =
            crate::parse_whole_input_inner_with_trace(self, src, TraceSession::new());
        trace.set_source_text(src);
        crate::write_trace_to_file(&trace, trace_path, format)?;
        match result {
            Ok((output, errors)) => Ok((output, errors)),
            Err(parse_err) => Err(crate::ParseWithTraceToFileError::Parse(parse_err)),
        }
    }
}

/// Combinator helpers for parsers defined in this crate (including macro-built [`crate::capture`] parsers).
///
/// See [`crate::guide::errors_and_recovery`] for `recover_with` / `add_error_info`, and
/// [`crate::guide::common_patterns`] for `erase_types` on large grammars.
pub trait ParserCombinator {
    /// Memoize parse results of type T keyed by input position (including outputs that borrow the input).
    fn memoized<T>(self) -> memoized::Memoized<Self, T>
    where
        Self: Sized,
    {
        memoized::Memoized::new(self)
    }

    /// On parse failure, run `recover_matcher` and yield `recover_output` if it matches.
    fn recover_with<RecoveryParser>(
        self,
        recover_parser: RecoveryParser,
    ) -> ErrorRecovererInner<Self, RecoveryParser>
    where
        Self: Sized,
    {
        ErrorRecovererInner::new(self, recover_parser)
    }

    #[cfg(feature = "parser-trace")]
    #[track_caller]
    /// Enrich hard failures from this parser with additional local context.
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    #[cfg(not(feature = "parser-trace"))]
    /// Enrich hard failures from this parser with additional local context.
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    #[cfg(feature = "parser-trace")]
    #[track_caller]
    /// Run this parser as a matcher and discard the output.
    fn ignore_result(self) -> IgnoreResult<Self>
    where
        Self: Sized,
    {
        IgnoreResult::new(self)
    }

    #[cfg(not(feature = "parser-trace"))]
    /// Run this parser as a matcher and discard the output.
    fn ignore_result(self) -> IgnoreResult<Self>
    where
        Self: Sized,
    {
        IgnoreResult::new(self)
    }

    /// Map each successful parser output with `map_fn`, preserving parse behavior.
    fn map_output<MapFn>(self, map_fn: MapFn) -> output_mapper::OutputMapper<Self, MapFn>
    where
        Self: Sized,
    {
        output_mapper::OutputMapper::new(self, map_fn)
    }

    /// Erase this parser's concrete type into a boxed trait object.
    ///
    /// This is useful when parser combinator types become very large and you
    /// want a stable, uniform type at the cost of dynamic dispatch.
    fn erase_types<'src, Inp, Out>(self) -> erase_types::Erased<'src, 'src, Inp, Out>
    where
        Self: Sized + Parser<'src, Inp, Output = Out> + 'src,
        Inp: Input<'src> + 'src,
        Out: 'src,
    {
        erase_types::erase(self)
    }
}

impl<'src, Inp: Input<'src>, P> Parser<'src, Inp> for P where
    P: internal::ParserImpl<'src, Inp> + 'src
{
}

pub(crate) trait ParserObjSafe<'src, Inp: Input<'src>, Output>: std::fmt::Debug {
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Output>, MatcherRunError>;

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>>;

    fn clone_boxed<'a>(&self) -> Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>
    where
        Self: 'a;
}

impl<'src, Inp: Input<'src>, Output, P> ParserObjSafe<'src, Inp, Output> for P
where
    P: internal::ParserImpl<'src, Inp, Output = Output>,
{
    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Output>, MatcherRunError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, input),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, input),
        }
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        <Self as internal::ParserImpl<'src, Inp>>::maybe_label(self)
    }

    fn clone_boxed<'a>(&self) -> Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<Inner> ParserCombinator for &Inner where Inner: ParserCombinator {}

// impl Parser for all types that deref to a parser
impl<'src, Inner, Inp: Input<'src>> internal::ParserImpl<'src, Inp> for &Inner
where
    Inner: Parser<'src, Inp>,
{
    type Output = <Inner as internal::ParserImpl<'src, Inp>>::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}
impl<Inner> ParserCombinator for Rc<Inner> where Inner: ParserCombinator {}

impl<'src, Inner, Inp: Input<'src>> internal::ParserImpl<'src, Inp> for Rc<Inner>
where
    Inner: Parser<'src, Inp>,
{
    type Output = <Inner as internal::ParserImpl<'src, Inp>>::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}
impl<Inner> ParserCombinator for Box<Inner> where Inner: ParserCombinator {}

impl<'src, Inner, Inp: Input<'src>> internal::ParserImpl<'src, Inp> for Box<Inner>
where
    Inner: Parser<'src, Inp>,
{
    type Output = <Inner as internal::ParserImpl<'src, Inp>>::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}
