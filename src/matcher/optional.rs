use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{MatchRunner, Matcher},
};

pub struct Optional<Match> {
    matcher: Match,
}

impl<Match> Optional<Match> {
    fn new(matcher: Match) -> Self {
        Self { matcher }
    }
}

pub fn optional<Match>(matcher: Match) -> Optional<Match> {
    Optional::new(matcher)
}

impl<Token, MRes, Match> super::internal::MatcherImpl<Token, MRes> for Optional<Match>
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
        if runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(true);
        }
        Ok(true)
    }
}
