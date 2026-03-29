use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
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
