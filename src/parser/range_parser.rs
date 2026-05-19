//! Parse a single token that lies in a Rust [`RangeBounds`] (or use [`Range`] / [`RangeInclusive`] directly as parsers).

use std::{
    fmt::{Debug, Display},
    ops::{Range, RangeBounds, RangeInclusive},
};

use crate::{
    context::ParserContext,
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, MatcherCombinator, internal::MatcherImpl}, parser::ParserCombinator,
};

/// Named wrapper holding a [`RangeBounds`] value; accepts one in-range token (same idea as the [`Range`] / [`RangeInclusive`] impls below).
#[derive(Clone, Debug)]
pub struct RangeParser<Range> {
    range: Range,
}

impl<Range> ParserCombinator for RangeParser<Range> 
{
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
    Range: RangeBounds<Token> + Clone,
    Token: PartialOrd + Clone,
    Range: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse(
        &self,
        _context: &mut ParserContext<'src>,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
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

impl<Token> ParserCombinator for Range<Token> 
{
}

impl<'src, Inp: Input<'src, Token = Token>, Token> super::internal::ParserImpl<'src, Inp>
    for Range<Token>
where
    Token: PartialOrd + Clone,
    Range<Token>: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse(
        &self,
        _context: &mut ParserContext<'src>,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
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

impl<Token> ParserCombinator for RangeInclusive<Token> 
{
}

impl<'src, Inp: Input<'src, Token = Token>, Token> super::internal::ParserImpl<'src, Inp>
    for RangeInclusive<Token>
where
    Token: PartialOrd + Clone,
    RangeInclusive<Token>: Debug,
{
    type Output = Token;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse(
        &self,
        _context: &mut ParserContext<'src>,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
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

impl<Token> MatcherCombinator for Range<Token> 
{
}

impl<'src, Inp: Input<'src, Token = Token>, Token, MRes> MatcherImpl<'src, Inp, MRes>
    for Range<Token>
where
    Token: PartialOrd + Clone + Display,
    Range<Token>: Debug,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
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

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(format!("{}..{}", self.start, self.end)))
    }
}

impl<Token> MatcherCombinator for RangeInclusive<Token> 
{
}

impl<'src, Inp: Input<'src, Token = Token>, Token, MRes> MatcherImpl<'src, Inp, MRes>
    for RangeInclusive<Token>
where
    Token: PartialOrd + Clone + Display,
    RangeInclusive<Token>: Debug,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
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

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(format!("{}..={}", self.start(), self.end())))
    }
}

// --- `&[u8]` input (`Token = &u8`): ranges over `u8` ---

impl<'src> super::internal::ParserImpl<'src, &'src [u8]> for RangeInclusive<u8> {
    type Output = u8;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse(
        &self,
        _context: &mut ParserContext<'src>,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        let old_pos = input.get_pos();
        if let Some(token) = input.next()
            && self.contains(token)
        {
            return Ok(Some(*token));
        }
        input.set_pos(old_pos);
        Ok(None)
    }
}

impl<'src> super::internal::ParserImpl<'src, &'src [u8]> for Range<u8> {
    type Output = u8;
    const CAN_FAIL: bool = true;

    #[inline]
    fn parse(
        &self,
        _context: &mut ParserContext<'src>,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        let old_pos = input.get_pos();
        if let Some(token) = input.next()
            && self.contains(token)
        {
            return Ok(Some(*token));
        }
        input.set_pos(old_pos);
        Ok(None)
    }
}

impl<'src, MRes> MatcherImpl<'src, &'src [u8], MRes> for RangeInclusive<u8> {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, &'src [u8], MRes = MRes>,
        'src: 'a,
    {
        if let Some(token) = input.next()
            && self.contains(token)
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(format!("{}..={}", self.start(), self.end())))
    }
}

impl<'src, MRes> MatcherImpl<'src, &'src [u8], MRes> for Range<u8> {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, &'src [u8], MRes = MRes>,
        'src: 'a,
    {
        if let Some(token) = input.next()
            && self.contains(token)
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        Some(Box::new(format!("{}..{}", self.start, self.end)))
    }
}
