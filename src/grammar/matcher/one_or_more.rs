use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
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

impl<'a, 'ctx, Match, Runner> Matcher<Runner> for OneOrMore<Match>
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
        // First match is mandatory — propagate the error if absent.
        if !runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(false);
        }
        // Remaining matches are optional (same as Multiple).
        while runner.run_match(&self.matcher, error_handler, pos)? {}
        Ok(true)
    }
}
