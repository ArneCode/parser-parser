pub mod multiple;
pub mod one_or_more;
pub mod token_parser;
use std::{ops::Deref, rc::Rc};

use crate::grammar::context::ParserContext;

pub trait Parser<T> {
    type Output;
    fn parse(&self, context: Rc<ParserContext<T>>, pos: &mut usize)
    -> Result<Self::Output, String>;
}

// impl Parser for all types that deref to a parser
impl<Token, Out, T, Pars> Parser<Token> for T
where
    T: Deref<Target = Pars>,
    Pars: Parser<Token, Output = Out>,
{
    type Output = Out;

    fn parse(
        &self,
        context: Rc<ParserContext<Token>>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        (**self).parse(context, pos)
    }
}
