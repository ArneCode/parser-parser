//! Parse a single token that lies in a Rust [`RangeBounds`] (or use [`Range`] / [`RangeInclusive`] directly as parsers).

use std::{
    fmt::{Debug, Display},
    ops::{Range, RangeBounds, RangeInclusive},
};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream}, matcher::{MatchRunner, internal::MatcherImpl},
};

/// Named wrapper holding a [`RangeBounds`] value; accepts one in-range token (same idea as the [`Range`] / [`RangeInclusive`] impls below).
pub struct RangeParser<Range> {
    range: Range,
}

impl<Range> RangeParser<Range> {
    /// Parser that accepts one token contained in `range`.
    pub fn new(range: Range) -> Self {
        Self { range }
    }
}

impl<'src, Inp: Input<'src, Token = Token>, Token, Range> super::internal::ParserImpl<'src, Inp>
    for RangeParser<Range>
where
    Range: RangeBounds<Token>,
    Token: PartialOrd + Clone,
    Range: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let old_pos = input.get_pos();
        if let Some(token) = input.next()
            && self.range.contains(&token)
        {
            return Ok(Some(token.clone()));
        }
        input.set_pos(old_pos);
        Ok(None)
    }
}

impl<'src, Inp: Input<'src, Token = Token>, Token> super::internal::ParserImpl<'src, Inp>
    for Range<Token>
where
    Token: PartialOrd + Clone,
    Range<Token>: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let old_pos = input.get_pos();
        if let Some(token) = input.next()
            && self.contains(&token)
        {
            return Ok(Some(token.clone()));
        }
        input.set_pos(old_pos);
        Ok(None)
    }
}

impl<'src, Inp: Input<'src, Token = Token>, Token> super::internal::ParserImpl<'src, Inp>
    for RangeInclusive<Token>
where
    Token: PartialOrd + Clone,
    Range<Token>: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let old_pos = input.get_pos();
        if let Some(token) = input.next()
            && self.contains(&token)
        {
            return Ok(Some(token.clone()));
        }
        input.set_pos(old_pos);
        Ok(None)
    }
}

impl<'src, Inp: Input<'src, Token = Token>, Token, MRes> MatcherImpl<'src, Inp, MRes> for Range<Token>
where
    Token: PartialOrd + Clone + Display,
    Range<Token>: Debug,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if let Some(token) = input.next()
            && self.contains(&token)
        {
            return Ok(true);
        }
        Ok(false)
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(
            format!("{}..{}", self.start, self.end)
        ))
    }
}

impl<'src, Inp: Input<'src, Token = Token>, Token, MRes> MatcherImpl<'src, Inp, MRes> for RangeInclusive<Token>
where
    Token: PartialOrd + Clone + Display,
    Range<Token>: Debug,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if let Some(token) = input.next()
            && self.contains(&token)
        {
            return Ok(true);
        }
        Ok(false)
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(
            format!("{}..={}", self.start(), self.end())
        ))
    }
}