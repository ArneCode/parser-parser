use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::Matcher,
};
pub struct OneOrMore<Match> {
    matcher: Match,
    id: usize,
}

impl<Match> OneOrMore<Match> {
    pub fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
        }
    }
}

/// e+  — match one or more repetitions of `matcher`, capturing each occurrence.
pub fn one_or_more<Match>(matcher: Match) -> OneOrMore<Match> {
    OneOrMore::new(matcher)
}

impl<Match> HasId for OneOrMore<Match>
where
    Match: HasId,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Match> IsCheckable<Token> for OneOrMore<Match>
where
    Match: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        // Must consume at least one token.
        if !self.matcher.check(context, pos) {
            return false;
        }
        // Greedily consume the rest (mirrors Multiple).
        while self.matcher.check(context, pos) {}
        true
    }
}

impl<Token, MRes, Match> Matcher<Token, MRes> for OneOrMore<Match>
where
    Match: Matcher<Token, MRes> + Grammar<Token>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        // First match is mandatory — propagate the error if absent.
        self.matcher.match_pattern(context, pos)?;
        // Remaining matches are optional (same as Multiple).
        while self.matcher.check_no_advance(context.parser_context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<Match, Label> MaybeLabel<Label> for OneOrMore<Match> {}
