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
pub use token_parser::{TokenParser, token_parser};
use std::rc::Rc;

use crate::{
    context::ParserContext,
    error::{
        FurthestFailError,
        error_handler::{ErrorHandler, ErrorHandlerChoice},
    },
    parser::recover_error::ErrorRecoverer as ErrorRecovererInner,
};

pub(crate) mod internal {
    use crate::{
        context::ParserContext,
        error::{FurthestFailError, error_handler::ErrorHandler},
    };

    pub trait ParserImpl<Token> {
        type Output;
        /// `true` when this parser can return `Ok(None)` on a normal parse path.
        ///
        /// This constant models parse absence and does not indicate whether
        /// `Err(FurthestFailError)` may be returned.
        const CAN_FAIL: bool;

        fn parse<'ctx>(
            &self,
            context: &mut ParserContext<'ctx, Token>,
            error_handler: &mut impl ErrorHandler,
            pos: &mut usize,
        ) -> Result<Option<Self::Output>, FurthestFailError>;
    }
}

pub trait Parser<Token>: internal::ParserImpl<Token> {
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

    fn memoized(self) -> memoized::Memoized<Self>
    where
        Self: Sized,
        Self::Output: 'static,
    {
        memoized::Memoized::new(self)
    }
}

impl<Token, P> Parser<Token> for P
where
    P: internal::ParserImpl<Token>,
{}

pub(crate) trait ParserObjSafe<Token> {
    type Output;
    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: ErrorHandlerChoice<'_>,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError>;
}

impl<Token, P> ParserObjSafe<Token> for P
where
    P: internal::ParserImpl<Token>,
{
    type Output = P::Output;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: ErrorHandlerChoice<'_>,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, pos),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, pos),
        }
    }
}

// impl Parser for all types that deref to a parser
impl<Inner, Token> internal::ParserImpl<Token> for &Inner
where
    Inner: Parser<Token>,
{
    type Output = Inner::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, pos)
    }
}
impl<Inner, Token> internal::ParserImpl<Token> for Rc<Inner>
where
    Inner: Parser<Token>,
{
    type Output = Inner::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, pos)
    }
}
impl<Inner, Token> internal::ParserImpl<Token> for Box<Inner>
where
    Inner: Parser<Token>,
{
    type Output = Inner::Output;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        (**self).parse(context, error_handler, pos)
    }
}
