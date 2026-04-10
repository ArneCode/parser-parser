use crate::grammar::{
    context::MatchResult,
    error_handler::{ErrorHandler, ParserError},
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner,
    },
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

impl<'a, 'ctx, Match, Runner> CanImplMatchWithRunner<Runner> for Multiple<Match>
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
        while runner.run_match(&self.matcher, error_handler, pos)? {}
        Ok(true)
    }
}

impl<Match> DoImplMatchWithNoMoemoizeBacktrackingRunner for Multiple<Match> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}
