//! Parser combinators: types that implement [`Parser`].
//!
//! You build parsers by composing the types in this module (and in
//! [`crate::one_of`]). [`Parser`] is not intended to be implemented outside this
//! crate: it extends a crate-private implementation trait, so only types
//! defined here can satisfy the full bound.
//!
//! ## Associated constants
//!
//! Implementations expose `CAN_FAIL`. When `true`, the parser may return `Ok(None)` on a normal path
//! (no match at the current position). It does **not** describe whether `Err` with
//! [`crate::error::MatcherRunError`] is possible on the crate-private [`internal::ParserImpl::parse`] path.

pub mod capture;
pub mod deferred;
pub mod erase_types;
pub(crate) mod fn_parser;
pub mod memoized;
pub mod multiple;
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
pub use memoized::Memoized;
pub use multiple::MultipleParser;
pub use range_parser::RangeParser;
pub use recover_error::ErrorRecoverer;
pub use single_token::SingleTokenParser;
use std::rc::Rc;
pub use token_parser::{TokenParser, token_parser};

use crate::{
    context::ParserContext,
    error::{
        FurthestFailError,
        MatcherRunError,
        ParserError,
        error_handler::{ErrorHandler, ErrorHandlerChoice},
    },
    input::{Input, InputStream},
    matcher::{ErrorContextualizer, ignore_result::IgnoreResult},
    parser::recover_error::ErrorRecoverer as ErrorRecovererInner,
};
#[cfg(feature = "parser-trace")]
use crate::trace::{TraceFormat, TraceSession};

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
            context: &mut ParserContext,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, Inp>,
        ) -> Result<Option<Self::Output>, MatcherRunError>;

        fn maybe_label(&self) -> Option<Box<dyn Display>> {
            None
        }
    }
}

/// Object-safe facade for parsers over a token type `Token`.
///
/// Blanket-implemented for every type that implements the crate-private parsing
/// trait used internally. Use [`recover_with`](Self::recover_with) and
/// [`memoized`](Self::memoized) for common extensions; the three-argument [`internal::ParserImpl::parse`]
/// method drives the actual parse step.
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
    fn parse_whole_input(&self, input: Inp) -> Result<
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
    fn parse_str(&self, src: &'src str) -> Result<
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
        let (result, trace) =
            crate::parse_whole_input_inner_with_trace(self, input, trace_session);
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

pub trait ParserCombinator {
    /// Memoize parse results keyed by input position (output type must be `'static`).
    fn memoized(self) -> memoized::Memoized<Self>
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
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    #[cfg(not(feature = "parser-trace"))]
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    #[cfg(feature = "parser-trace")]
    #[track_caller]
    fn ignore_result(self) -> IgnoreResult<Self>
    where
        Self: Sized,
    {
        IgnoreResult::new(self)
    }

    #[cfg(not(feature = "parser-trace"))]
    fn ignore_result(self) -> IgnoreResult<Self>
    where
        Self: Sized,
    {
        IgnoreResult::new(self)
    }

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

    /// Conditionally erase parser types based on the `parser-erased` feature.
    ///
    /// - With `parser-erased` enabled, this behaves like [`Self::erase_types`]
    ///   and returns an erased parser.
    /// - Without the feature, this is a no-op and returns `Self`.
    ///
    /// This keeps call sites stable while allowing opt-in type erasure from
    /// Cargo features instead of debug/release profile differences.
    #[cfg(feature = "parser-erased")]
    fn maybe_erase_types<'src, Inp, Out>(self) -> erase_types::Erased<'src, 'src, Inp, Out>
    where
        Self: Sized + Parser<'src, Inp, Output = Out> + 'src,
        Inp: Input<'src> + 'src,
        Out: 'src,
    {
        erase_types::erase(self)
    }

    /// See the `parser-erased` variant of this method for behavior details.
    #[cfg(not(feature = "parser-erased"))]
    fn maybe_erase_types(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl<'src, Inp: Input<'src>, P> Parser<'src, Inp> for P where P: internal::ParserImpl<'src, Inp> + 'src {}

pub(crate) trait ParserObjSafe<'src, Inp: Input<'src>, Output>: std::fmt::Debug {
    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Output>, MatcherRunError>;

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>>;

    fn clone_boxed<'a>(self: &Self) -> Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>
    where
        Self: 'a;
}

impl<'src, Inp: Input<'src>, Output, P> ParserObjSafe<'src, Inp, Output> for P
where
    P: internal::ParserImpl<'src, Inp, Output = Output>,
{
    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Output>, MatcherRunError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, input),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, input),
        }
    }

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        <Self as internal::ParserImpl<'src, Inp>>::maybe_label(self)
    }

    fn clone_boxed<'a>(self: &Self) -> Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>
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

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

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

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

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

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        (**self).parse(context, error_handler, input)
    }

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}
