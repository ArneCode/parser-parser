use std::marker::PhantomData;

use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
};

pub struct Multiple<MRes, Match> {
    matcher: Match,
    id: usize,
    _phantom: PhantomData<MRes>,
}

impl<Match, MRes> Multiple<MRes, Match> {
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn many<MRes, Match>(matcher: Match) -> Multiple<MRes, Match> {
    Multiple::new(matcher)
}
impl<Match, Token, MRes> Matcher<Token, MRes> for Multiple<MRes, Match>
where
    Match: Matcher<Token, MRes> + HasId + IsCheckable<Token>,
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

impl<MRes, T, Match> IsCheckable<T> for Multiple<MRes, Match>
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

impl<MRes, Match> HasId for Multiple<MRes, Match> {
    fn id(&self) -> usize {
        self.id
    }
}
