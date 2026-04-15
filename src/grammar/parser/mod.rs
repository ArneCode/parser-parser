pub mod multiple;
pub mod one_of;
// pub mod one_or_more;
pub mod deferred;
pub mod range_parser;
pub mod recover_error;
pub mod single_token;
pub mod token_parser;
use std::rc::Rc;

use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ErrorHandlerChoice, ParserError},
    parser::recover_error::ErrorRecoverer,
};

pub trait Parser<Token> {
    type Output;
    const CAN_FAIL: bool;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError>;
    fn recover_with<Match, Output>(
        self,
        recover_matcher: Match,
        recover_output: Output,
    ) -> ErrorRecoverer<Self, Match, Output>
    where
        Self: Sized,
    {
        ErrorRecoverer::new(self, recover_matcher, recover_output)
    }
}
pub(crate) trait ParserObjSafe<Token> {
    type Output;
    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: ErrorHandlerChoice<'_>,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError>;
}

impl<Token, P> ParserObjSafe<Token> for P
where
    P: Parser<Token>,
{
    type Output = P::Output;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: ErrorHandlerChoice<'_>,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        match error_handler {
            ErrorHandlerChoice::Empty(handler) => self.parse(context, handler, pos),
            ErrorHandlerChoice::Multi(handler) => self.parse(context, handler, pos),
        }
    }
}

// impl Parser for all types that deref to a parser
impl<Inner, Token> Parser<Token> for &Inner
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
    ) -> Result<Option<Self::Output>, ParserError> {
        (**self).parse(context, error_handler, pos)
    }
}
impl<Inner, Token> Parser<Token> for Rc<Inner>
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
    ) -> Result<Option<Self::Output>, ParserError> {
        (**self).parse(context, error_handler, pos)
    }
}
impl<Inner, Token> Parser<Token> for Box<Inner>
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
    ) -> Result<Option<Self::Output>, ParserError> {
        (**self).parse(context, error_handler, pos)
    }
}

pub trait ToParser {
    type ParserType;
    fn to_parser(&self) -> Self::ParserType;
}
