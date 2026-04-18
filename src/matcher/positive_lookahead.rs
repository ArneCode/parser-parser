use crate::{error::error_handler::ErrorHandler, matcher::Matcher};
pub struct PositiveLookahead<Check> {
    checker: Check,
}

impl<Check> PositiveLookahead<Check> {
    pub fn new(checker: Check) -> Self {
        Self { checker }
    }
}

/// &e  — positive lookahead. Succeeds without consuming if `e` would match.
pub fn positive_lookahead<Check>(checker: Check) -> PositiveLookahead<Check> {
    PositiveLookahead::new(checker)
}

impl<Token, MRes, Check> super::internal::MatcherImpl<Token, MRes> for PositiveLookahead<Check>
where
    Check: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Check::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = Check::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: crate::matcher::MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        let mut original_pos = *pos;
        let result = self
            .checker
            .match_with_runner(runner, error_handler, &mut original_pos)?;
        Ok(result)
    }
}
