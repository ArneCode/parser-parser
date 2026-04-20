//! Predicate-based token parser: `check_fn` gates consumption; `parse_fn` maps the token to output.

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
};

/// [`crate::parser::Parser`] built from a predicate and a projection function.
pub struct TokenParser<CheckF, ParseF> {
    check_fn: CheckF,
    parse_fn: ParseF,
}

impl<CheckF, ParseF> TokenParser<CheckF, ParseF> {
    /// See [`token_parser`].
    pub fn new<Token, Out>(check_fn: CheckF, parse_fn: ParseF) -> Self
    where
        CheckF: Fn(&Token) -> bool,
        ParseF: Fn(&Token) -> Out,
    {
        Self { check_fn, parse_fn }
    }
}

/// Convenience constructor for [`TokenParser`].
pub fn token_parser<CheckF, ParseF, Token, Out>(
    check_fn: CheckF,
    parse_fn: ParseF,
) -> TokenParser<CheckF, ParseF>
where
    CheckF: Fn(&Token) -> bool,
    ParseF: Fn(&Token) -> Out,
{
    TokenParser::new(check_fn, parse_fn)
}

impl<'src, Inp: Input<'src>, Out, CheckF, ParseF> super::internal::ParserImpl<'src, Inp>
    for TokenParser<CheckF, ParseF>
where
    CheckF: Fn(&<Inp as Input<'src>>::Token) -> bool,
    ParseF: Fn(&<Inp as Input<'src>>::Token) -> Out,
{
    type Output = Out;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start = input.get_pos();
        let Some(token) = input.next() else {
            return Ok(None);
        };
        if (self.check_fn)(&token) {
            return Ok(Some((self.parse_fn)(&token)));
        }
        input.set_pos(start);
        Ok(None)
    }
}
