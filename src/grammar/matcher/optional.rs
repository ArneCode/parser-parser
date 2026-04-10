use crate::grammar::{
    context::ParserContext,
    error_handler::{self, ErrorHandler, ParserError},
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner,
    },
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

impl<'a, 'ctx, Match, Runner> CanImplMatchWithRunner<Runner> for Optional<Match>
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
        if runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(true);
        }
        Ok(true)
    }
}

impl<Match> DoImplMatchWithNoMoemoizeBacktrackingRunner for Optional<Match> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}
