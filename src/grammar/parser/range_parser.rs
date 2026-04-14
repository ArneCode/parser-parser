use std::{fmt::Debug, ops::RangeBounds};

use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    parser::Parser,
};

pub struct RangeParser<Range> {
    range: Range,
}

impl<Range> RangeParser<Range> {
    pub fn new(range: Range) -> Self {
        Self { range }
    }
}

impl<Token, Range> Parser<Token> for RangeParser<Range>
where
    Range: RangeBounds<Token>,
    Token: PartialOrd + Clone,
    Range: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext<Token>,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        let token = context.tokens.get(*pos);
        if let Some(token) = token
            && self.range.contains(token)
        {
            *pos += 1;
            return Ok(Some(token.clone()));
        }
        Ok(None)
    }
}
