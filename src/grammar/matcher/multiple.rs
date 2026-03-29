use std::{marker::PhantomData, ops::Deref};

use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
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
