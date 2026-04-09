pub mod multiple;
pub mod one_of;
pub mod one_or_more;
pub mod range_parser;
pub mod single_token;
pub mod token_parser;
use std::ops::Deref;

use crate::grammar::{context::ParserContext, error_handler::ErrorHandler};

pub trait Parser<Token> {
    type Output;
    fn parse(
        &self,
        context: &mut ParserContext<'_, Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String>;
}

// impl Parser for all types that deref to a parser
impl<Inner, Outer, Token> Parser<Token> for Outer
where
    Outer: Deref<Target = Inner>,
    Inner: Parser<Token>,
{
    type Output = Inner::Output;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        (**self).parse(context, pos)
    }
}

pub trait ToParser {
    type ParserType;
    fn to_parser(&self) -> Self::ParserType;
}
