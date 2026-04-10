use crate::grammar::{
    context::ParserContext,
    error_handler::{self, ErrorHandler, ParserError},
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner,
    },
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

impl<'a, 'ctx, Match, Runner> CanImplMatchWithRunner<Runner> for OneOrMore<Match>
where
    Match: CanMatchWithRunner<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
{
    fn impl_match_with_runner(
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

impl<Match> DoImplMatchWithNoMoemoizeBacktrackingRunner for OneOrMore<Match> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}
