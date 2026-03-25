use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    checker: Check,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Check> NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// !e  — negative lookahead. Succeeds without consuming if `e` would *not* match.
pub fn negative_lookahead<T, N, Check>(checker: Check) -> NegativeLookahead<T, N, Check>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Check: HasId + IsCheckable<T> + 'static,
{
    NegativeLookahead::new(checker)
}

impl<T, N, Check> HasId for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Check> IsCheckable<T> for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Peek — pos must not move.  Success means the inner check *failed*.
        !self.checker.check_no_advance(context, pos)
    }
}

impl<T, N, Check> Matcher<T> for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if !self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!(
                "negative lookahead failed: forbidden pattern matched at position {}",
                pos
            ))
        }
    }
}
