use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
};
pub struct NegativeLookahead<Check> {
    checker: Check,
    id: usize,
}

impl<Check> NegativeLookahead<Check> {
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
        }
    }
}

pub fn negative_lookahead<Check>(checker: Check) -> NegativeLookahead<Check> {
    NegativeLookahead::new(checker)
}

impl<Check> HasId for NegativeLookahead<Check> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Check> IsCheckable<Token> for NegativeLookahead<Check>
where
    Check: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        // Peek — pos must not move.  Success means the inner check *failed*.
        !self.checker.check_no_advance(context, pos)
    }
}

impl<Token, MRes, Check> Matcher<Token, MRes> for NegativeLookahead<Check>
where
    Check: Grammar<Token>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if !self.checker.check_no_advance(context.parser_context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!(
                "negative lookahead failed: forbidden pattern matched at position {}",
                pos
            ))
        }
    }
}
