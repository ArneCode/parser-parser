use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::Matcher,
};
pub struct PositiveLookahead<Check> {
    checker: Check,
    id: usize,
}

impl<Check> PositiveLookahead<Check> {
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
        }
    }
}

/// &e  — positive lookahead. Succeeds without consuming if `e` would match.
pub fn positive_lookahead<Check>(checker: Check) -> PositiveLookahead<Check> {
    PositiveLookahead::new(checker)
}

impl<Check> HasId for PositiveLookahead<Check> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Check> IsCheckable<Token> for PositiveLookahead<Check>
where
    Check: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        // Pure peek — pos must not move regardless of outcome.
        self.checker.check_no_advance(context, pos)
    }
}

impl<Token, MRes, Check> Matcher<Token, MRes> for PositiveLookahead<Check>
where
    Check: Grammar<Token>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.checker.check_no_advance(context.parser_context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!("positive lookahead failed at position {}", pos))
        }
    }
}

impl<Label, Check> MaybeLabel<Label> for PositiveLookahead<Check> {}
