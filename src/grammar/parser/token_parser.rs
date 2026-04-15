use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    parser::Parser,
};
pub struct TokenParser<CheckF, ParseF> {
    check_fn: CheckF,
    parse_fn: ParseF,
}

impl<CheckF, ParseF> TokenParser<CheckF, ParseF> {
    pub fn new<Token, Out>(check_fn: CheckF, parse_fn: ParseF) -> Self
    where
        CheckF: Fn(&Token) -> bool,
        ParseF: Fn(&Token) -> Out,
    {
        Self { check_fn, parse_fn }
    }
}

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

impl<Token, Out, CheckF, ParseF> Parser<Token> for TokenParser<CheckF, ParseF>
where
    CheckF: for<'a> Fn(&'a Token) -> bool,
    ParseF: Fn(&Token) -> Out,
{
    type Output = Out;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext<Token>,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        if *pos < context.tokens.len() && (self.check_fn)(&context.tokens[*pos]) {
            let token = &context.tokens[*pos];
            *pos += 1; // Advance position on success
            Ok(Some((self.parse_fn)(token)))
        } else {
            Ok(None)
        }
    }
}
