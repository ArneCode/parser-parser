//! Exact single-token parsers: a literal value or a [`char`] literal.

use std::fmt::Debug;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream}, parser::ParserCombinator,
};

/// Matches `token` and advances by one on success.
#[derive(Clone, Debug)]
pub struct SingleTokenParser<Token> {
    token: Token,
}

impl<Token> SingleTokenParser<Token> {
    /// Parser for one occurrence of `token`.
    pub fn new(token: Token) -> Self {
        Self { token }
    }
}

impl<Token> ParserCombinator for SingleTokenParser<Token> 
{
}

impl<'src, Inp: Input<'src, Token = Token>, Token: PartialEq + Clone + Debug>
    super::internal::ParserImpl<'src, Inp> for SingleTokenParser<Token>
{
    type Output = Token;
    const CAN_FAIL: bool = true;
    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start = input.get_pos();
        if let Some(token) = input.next()
            && token == self.token
        {
            return Ok(Some(self.token.clone()));
        }
        input.set_pos(start);
        Ok(None)
    }
}

// impl<Token, Label> MaybeLabel<Label> for SingleTokenParser<Token> {}
impl ParserCombinator for char 
{
}

impl<'src, Inp: Input<'src, Token = char>> super::internal::ParserImpl<'src, Inp> for char {
    type Output = char;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start = input.get_pos();
        if let Some(token) = input.next()
            && token == *self
        {
            return Ok(Some(*self));
        }
        input.set_pos(start);
        Ok(None)
    }
}
