use crate::{
    error::error_handler::ErrorHandler,
    matcher::{MatchRunner, Matcher},
};
pub struct NegativeLookahead<Check> {
    checker: Check,
}

impl<Check> NegativeLookahead<Check> {
    pub fn new(checker: Check) -> Self {
        Self { checker }
    }
}

pub fn negative_lookahead<Check>(checker: Check) -> NegativeLookahead<Check> {
    NegativeLookahead::new(checker)
}

impl<Token, MRes, Match> super::internal::MatcherImpl<Token, MRes> for NegativeLookahead<Match>
where
    Match: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;

    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;

    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
    {
        // Peek — pos must not move.  Success means the inner check *failed*.
        let mut original_pos = *pos;
        let can_match = runner.run_match(&self.checker, error_handler, &mut original_pos)?;
        Ok(!can_match)
    }
}
