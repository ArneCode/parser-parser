use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatchResult, MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner, Matcher, NoMoemoizeBacktrackingRunner,
    },
};

pub struct Multiple<Match> {
    matcher: Match,
    id: usize,
}

impl<Match> Multiple<Match> {
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
        }
    }
}

pub fn many<Match>(matcher: Match) -> Multiple<Match> {
    Multiple::new(matcher)
}
impl<Match, Token, MRes> Matcher<Token, MRes> for Multiple<Match>
where
    Match: Matcher<Token, MRes> + Grammar<Token>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        while self.matcher.check_no_advance(context.parser_context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<'a, 'ctx, Match, Runner> CanImplMatchWithRunner<Runner> for Multiple<Match>
where
    Match: CanMatchWithRunner<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
{
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String> {
        while runner.run_match(&self.matcher, pos)? {}
        Ok(true)
    }
}

impl<Match> DoImplMatchWithNoMoemoizeBacktrackingRunner for Multiple<Match> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}

impl<T, Match> IsCheckable<T> for Multiple<Match>
where
    Match: Grammar<T>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<T, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        while self.matcher.check(context, pos) {}
        true
    }
}

impl<Match> HasId for Multiple<Match> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Label, Match> MaybeLabel<Label> for Multiple<Match> {}
