use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner, Matcher,
    },
};

pub struct Optional<Match> {
    matcher: Match,
    id: usize,
}

impl<Match> Optional<Match> {
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
        }
    }
}

pub fn optional<Match>(matcher: Match) -> Optional<Match> {
    Optional::new(matcher)
}

impl<Token, MRes, Match> Matcher<Token, MRes> for Optional<Match>
where
    Match: Matcher<Token, MRes> + Grammar<Token>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.matcher.check_no_advance(context.parser_context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<'a, 'ctx, Match, Runner> CanImplMatchWithRunner<Runner> for Optional<Match>
where
    Match: CanMatchWithRunner<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
{
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String> {
        if runner.run_match(&self.matcher, pos)? {
            return Ok(true);
        }
        Ok(true)
    }
}

impl<Match> DoImplMatchWithNoMoemoizeBacktrackingRunner for Optional<Match> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}

impl<Token, Match> IsCheckable<Token> for Optional<Match>
where
    Match: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        self.matcher.check(context, pos);
        true
    }
}

impl<Match> HasId for Optional<Match> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Label, Match> MaybeLabel<Label> for Optional<Match> {}
