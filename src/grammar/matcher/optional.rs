use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
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

impl<'a, 'ctx, Match, Runner> Matcher<Runner> for Optional<Match>
where
    Match: Matcher<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = false;

    fn match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
        if runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(true);
        }
        Ok(true)
    }
}
