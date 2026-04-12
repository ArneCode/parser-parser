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
    pub fn new(check_fn: CheckF, parse_fn: ParseF) -> Self {
        Self { check_fn, parse_fn }
    }
}

// impl<CheckF, ParseF> HasId for TokenParser<CheckF, ParseF> {
//     fn id(&self) -> usize {
//         self.id
//     }
// }

// impl<Token, CheckF, ParseF> IsCheckable<Token> for TokenParser<CheckF, ParseF>
// where
//     CheckF: Fn(&Token) -> bool,
// {
//     fn calc_check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool {
//         if *pos < context.tokens.len() {
//             let token = &context.tokens[*pos];
//             *pos += 1; // Advance position on success
//             (self.check_fn)(token)
//         } else {
//             false
//         }
//     }
// }

impl<'ctx, Token, Out, CheckF, ParseF> Parser<'ctx, Token> for TokenParser<CheckF, ParseF>
where
    CheckF: Fn(&Token) -> bool,
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
