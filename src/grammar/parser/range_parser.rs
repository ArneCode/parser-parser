use std::{fmt::Debug, ops::RangeBounds};

use crate::grammar::{
    HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    label::MaybeLabel, parser::Parser,
};

pub struct RangeParser<Range> {
    range: Range,
    id: usize,
}

impl<Range> RangeParser<Range> {
    pub fn new(range: Range) -> Self {
        Self {
            range,
            id: get_next_id(),
        }
    }
}

impl<Range> HasId for RangeParser<Range> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Range> IsCheckable<Token> for RangeParser<Range>
where
    Range: RangeBounds<Token>,
    Token: PartialOrd,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        let token = context.tokens.get(*pos);
        if let Some(token) = token
            && self.range.contains(token) {
                *pos += 1;
                return true;
            }
        false
    }
}

impl<Token, Range> Parser<Token> for RangeParser<Range>
where
    Range: RangeBounds<Token>,
    Token: PartialOrd + Clone,
    Range: Debug,
{
    type Output = Token;

    fn parse(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let token = context.tokens.get(*pos);
        if let Some(token) = token
            && self.range.contains(token) {
                *pos += 1;
                return Ok(token.clone());
            }
        Err(format!(
            "Expected token in range {:?} at position {}",
            self.range, pos
        ))
    }
}

impl<Range> MaybeLabel<String> for RangeParser<Range>
where
    Range: Debug,
{
    // fn maybe_label(&self) -> Option<String> {
    //     Some(format!("{:?}", self.range))
    // }
}
