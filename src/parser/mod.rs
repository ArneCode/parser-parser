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
pub mod memoized;
pub mod multiple;
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
    input::{InputFamily, InputStream},
    parser::recover_error::ErrorRecoverer as ErrorRecovererInner,
};

pub(crate) mod internal {
    use crate::{
        context::ParserContext,
        error::{FurthestFailError, error_handler::ErrorHandler},
        input::{InputFamily, InputStream},
    };

    /// Crate-private parsing interface used by [`super::Parser`].
    pub trait ParserImpl<InpFam>
    where
        InpFam: InputFamily + ?Sized,
    {
        /// Successful parse value when the parser matches at `pos`.
        type Output<'src>;
        /// `true` when this parser can return `Ok(None)` on a normal parse path.
        ///
        /// This constant models parse absence and does not indicate whether
        /// `Err(FurthestFailError)` may be returned.
        const CAN_FAIL: bool;

        /// Run the parser at `pos` against `context`, reporting secondary issues through `error_handler`.
        fn parse<'src>(
            &self,
            context: &mut ParserContext,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, InpFam::In<'src>>,
        ) -> Result<Option<Self::Output<'src>>, FurthestFailError>;
    }
}

/// Object-safe facade for parsers over a token type `Token`.
///
/// Blanket-implemented for every type that implements the crate-private parsing
/// trait used internally. Use [`recover_with`](Self::recover_with) and
/// [`memoized`](Self::memoized) for common extensions; the `parse` method is
/// inherited from that internal trait and drives the actual parse step.
pub trait Parser<InpFam>: internal::ParserImpl<InpFam>
where
    InpFam: InputFamily + ?Sized,
{
    /// On parse failure, run `recover_matcher` and yield `recover_output` if it matches.
    fn recover_with<Match, Output>(
        self,
        recover_matcher: Match,
        recover_output: Output,
    ) -> ErrorRecovererInner<Self, Match, Output>
    where
        Self: Sized,
    {
        ErrorRecovererInner::new(self, recover_matcher, recover_output)
    }

    /// Memoize parse results keyed by input position (output type must be `'static`).
    fn memoized(self) -> memoized::Memoized<Self>
    where
        Self: Sized,
        for<'src> Self::Output<'src>: 'static,
    {
        memoized::Memoized::new(self)
    }
}

impl<InpFam, P> Parser<InpFam> for P
where
    InpFam: InputFamily + ?Sized,
    P: internal::ParserImpl<InpFam>,
{
}

pub(crate) trait ParserObjSafe<InpFam, Output>
where
    InpFam: InputFamily + ?Sized,
{
    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Output>, FurthestFailError>;
}

impl<InpFam, Output, P> ParserObjSafe<InpFam, Output> for P
where
    InpFam: InputFamily + ?Sized,
    P: for<'src> internal::ParserImpl<InpFam, Output<'src> = Output>,
{
    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: ErrorHandlerChoice<'_>,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Output>, FurthestFailError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, input),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, input),
        }
    }
}

// impl Parser for all types that deref to a parser
impl<Inner, InpFam> internal::ParserImpl<InpFam> for &Inner
where
    InpFam: InputFamily + ?Sized,
    Inner: Parser<InpFam>,
{
    type Output<'src> = Inner::Output<'src>;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
    }
}
impl<Inner, InpFam> internal::ParserImpl<InpFam> for Rc<Inner>
where
    InpFam: InputFamily + ?Sized,
    Inner: Parser<InpFam>,
{
    type Output<'src> = Inner::Output<'src>;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
    }
}
impl<Inner, InpFam> internal::ParserImpl<InpFam> for Box<Inner>
where
    InpFam: InputFamily + ?Sized,
    Inner: Parser<InpFam>,
{
    type Output<'src> = Inner::Output<'src>;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
        (**self).parse(context, error_handler, input)
    }
}

