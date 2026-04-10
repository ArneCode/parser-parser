// pub mod multiple;
pub mod one_of;
// pub mod one_or_more;
pub mod range_parser;
pub mod single_token;
pub mod token_parser;
use std::ops::Deref;

use crate::grammar::{
    context::ParserContext,
    error_handler::{self, ErrorHandler, ParserError},
};

pub trait Parser<'ctx, Token> {
    type Output;
    fn parse(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError>;
}

// impl Parser for all types that deref to a parser
impl<'ctx, Inner, Outer, Token> Parser<'ctx, Token> for Outer
where
    Outer: Deref<Target = Inner>,
    Inner: Parser<'ctx, Token>,
{
    type Output = Inner::Output;

    fn parse(
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
