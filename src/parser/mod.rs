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
//! [`crate::error::FurthestFailError`] is possible.

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
    ResultBinder, SingleProperty, SpanBinder, bind_result, bind_result_with_debug,
    bind_result_with_unknown_debug, bind_span,
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
        error_handler::{ErrorHandler, ErrorHandlerChoice},
    },
    input::{Input, InputStream},
    matcher::{ErrorContextualizer, ignore_result::IgnoreResult},
    parser::recover_error::ErrorRecoverer as ErrorRecovererInner,
};

pub(crate) mod internal {
    use std::fmt::Debug;

    use crate::{
        context::ParserContext,
        error::{FurthestFailError, error_handler::ErrorHandler},
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
        /// `Err(FurthestFailError)` may be returned.
        const CAN_FAIL: bool;

        /// Run the parser at `pos` against `context`, reporting secondary issues through `error_handler`.
        fn parse(
            &self,
            context: &mut ParserContext,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, Inp>,
        ) -> Result<Option<Self::Output>, FurthestFailError>;
    }
}

/// Object-safe facade for parsers over a token type `Token`.
///
/// Blanket-implemented for every type that implements the crate-private parsing
/// trait used internally. Use [`recover_with`](Self::recover_with) and
/// [`memoized`](Self::memoized) for common extensions; the `parse` method is
/// inherited from that internal trait and drives the actual parse step.
pub trait Parser<'src, Inp: Input<'src>>: internal::ParserImpl<'src, Inp> {}

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

    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

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
    fn erase_types<'a, 'src, Inp, Out>(self) -> erase_types::Erased<'a, 'src, Inp, Out>
    where
        Self: Sized + Parser<'src, Inp, Output = Out> + 'a,
        Inp: Input<'src> + 'a,
        Out: 'a,
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
    fn maybe_erase_types<'a, 'src, Inp, Out>(self) -> erase_types::Erased<'a, 'src, Inp, Out>
    where
        Self: Sized + Parser<'src, Inp, Output = Out> + 'a,
        Inp: Input<'src> + 'a,
        Out: 'a,
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

impl<'src, Inp: Input<'src>, P> Parser<'src, Inp> for P where P: internal::ParserImpl<'src, Inp> {}

pub(crate) trait ParserObjSafe<'src, Inp: Input<'src>, Output>: std::fmt::Debug {
    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Output>, FurthestFailError>;

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
    ) -> Result<Option<Output>, FurthestFailError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, input),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, input),
        }
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
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
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
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
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
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
    }
}
