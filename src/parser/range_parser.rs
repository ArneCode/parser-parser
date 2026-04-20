//! Parse a single token that lies in a Rust [`RangeBounds`] (or use [`Range`] / [`RangeInclusive`] directly as parsers).

use std::{
    fmt::Debug,
    ops::{Range, RangeBounds, RangeInclusive},
};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
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

impl<InpFam, Token, Range> super::internal::ParserImpl<InpFam> for RangeParser<Range>
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = Token>,
    Range: RangeBounds<Token>,
    Token: PartialOrd + Clone,
    Range: Debug,
{
    type Output<'src> = Token;
    const CAN_FAIL: bool = true;

    fn parse<'src>(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
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

impl<InpFam, Token> super::internal::ParserImpl<InpFam> for Range<Token>
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = Token>,
    Token: PartialOrd + Clone,
    Range<Token>: Debug,
{
    type Output<'src> = Token;
    const CAN_FAIL: bool = true;

    fn parse<'src>(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
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

impl<InpFam, Token> super::internal::ParserImpl<InpFam> for RangeInclusive<Token>
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = Token>,
    Token: PartialOrd + Clone,
    Range<Token>: Debug,
{
    type Output<'src> = Token;
    const CAN_FAIL: bool = true;

    fn parse<'src>(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
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
