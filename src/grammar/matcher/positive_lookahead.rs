use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};
pub struct PositiveLookahead<T, Check> {
    checker: Check,
    id: usize,
    _phantom: PhantomData<T>,
}

impl<T, Check> PositiveLookahead<T, Check>
where
    Check: Grammar<T>,
{
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// &e  — positive lookahead. Succeeds without consuming if `e` would match.
pub fn positive_lookahead<T, Check>(checker: Check) -> PositiveLookahead<T, Check>
where
    Check: Grammar<T>,
{
    PositiveLookahead::new(checker)
}

impl<T, Check> HasId for PositiveLookahead<T, Check> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, Check> IsCheckable<T> for PositiveLookahead<T, Check>
where
    Check: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Pure peek — pos must not move regardless of outcome.
        self.checker.check_no_advance(context, pos)
    }
}

impl<T, MContext, Check> Matcher<T, MContext> for PositiveLookahead<T, Check>
where
    MContext: Deref<Target = ParserContext<T>>,
    Check: HasId + IsCheckable<T>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        if self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!("positive lookahead failed at position {}", pos))
        }
    }
}
