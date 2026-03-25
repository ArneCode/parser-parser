pub mod multiple;
pub mod one_or_more;
pub mod token_parser;
use std::rc::Rc;

use crate::grammar::{AstNode, Token, context::ParserContext};

pub trait Parser<T: Token> {
    type Output: AstNode + ?Sized;
    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String>;
}

// impl Parser for all Rc<Parser>
impl<T, N, P> Parser<T> for Rc<P>
where
    T: Token,
    N: AstNode + ?Sized,
    P: Parser<T, Output = N>,
{
    type Output = N;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        (**self).parse(context, pos)
    }
}
