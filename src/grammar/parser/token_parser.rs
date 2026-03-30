use std::marker::PhantomData;

use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    parser::Parser,
};
pub struct TokenParser<CheckF, ParseF, Token, Out> {
    check_fn: CheckF,
    parse_fn: ParseF,
    id: usize,
    _phantom: PhantomData<(Token, Out)>,
}

impl<CheckF, ParseF, Token, Out> TokenParser<CheckF, ParseF, Token, Out>
where
    CheckF: Fn(&Token) -> bool,
    ParseF: Fn(&Token) -> Out,
{
    pub fn new(check_fn: CheckF, parse_fn: ParseF) -> Self {
        Self {
            check_fn,
            parse_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<CheckF, ParseF, Out, Token> HasId for TokenParser<CheckF, ParseF, Token, Out> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, CheckF, ParseF, Out> IsCheckable<Token> for TokenParser<CheckF, ParseF, Token, Out>
where
    CheckF: Fn(&Token) -> bool,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        if *pos < context.tokens.len() {
            let token = &context.tokens[*pos];
            *pos += 1; // Advance position on success
            (self.check_fn)(token)
        } else {
            false
        }
    }
}

impl<Token, Out, CheckF, ParseF> Parser<Token> for TokenParser<CheckF, ParseF, Token, Out>
where
    CheckF: Fn(&Token) -> bool,
    ParseF: Fn(&Token) -> Out,
{
    type Output = Out;

    fn parse(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        if *pos < context.tokens.len() {
            if self.check_no_advance(context, pos) {
                let token = &context.tokens[*pos];
                *pos += 1; // Advance position on success
                Ok((self.parse_fn)(token))
            } else {
                Err(format!(
                    "token did not satisfy check function at position {}",
                    pos
                ))
            }
        } else {
            Err(format!(
                "expected token at position {} but reached end of input",
                pos
            ))
        }
    }
}
