use crate::grammar::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{MatchRunner, Matcher},
};
pub struct OneOrMore<Match> {
    matcher: Match,
}

impl<Match> OneOrMore<Match> {
    pub fn new(matcher: Match) -> Self {
        Self { matcher }
    }
}

/// e+  — match one or more repetitions of `matcher`, capturing each occurrence.
pub fn one_or_more<Match>(matcher: Match) -> OneOrMore<Match> {
    OneOrMore::new(matcher)
}

impl<Token, MRes, Match> Matcher<Token, MRes> for OneOrMore<Match>
where
    Match: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = false;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
    {
        // First match is mandatory — propagate the error if absent.
        if !runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(false);
        }
        // Remaining matches are optional (same as Multiple).
        while runner.run_match(&self.matcher, error_handler, pos)? {}
        Ok(true)
    }
}
