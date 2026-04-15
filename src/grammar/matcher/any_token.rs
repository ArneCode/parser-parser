use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
};
pub struct AnyToken;

impl<Token, MRes> Matcher<Token, MRes> for AnyToken {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        let context = runner.get_parser_context();
        if *pos < context.tokens.len() {
            *pos += 1; // Advance position
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
