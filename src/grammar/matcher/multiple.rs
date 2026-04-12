use crate::grammar::{
    capture::CanNotFail,
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
};

pub struct Multiple<Match> {
    matcher: Match,
}

impl<Match> Multiple<Match> {
    fn new(matcher: Match) -> Self {
        Self { matcher }
    }
}

pub fn many<Match>(matcher: Match) -> Multiple<Match> {
    Multiple::new(matcher)
}

// impl<Match> Matcher for Multiple<Match> where Match: Matcher {}

impl<'a, 'ctx, Match, Runner> Matcher<Runner> for Multiple<Match>
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
        while runner.run_match(&self.matcher, error_handler, pos)? {}
        Ok(true)
    }
}

impl<Match> CanNotFail for Multiple<Match> {}
