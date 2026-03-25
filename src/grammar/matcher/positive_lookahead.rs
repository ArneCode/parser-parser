use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
{
    checker: Check,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Check> PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
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
pub fn positive_lookahead<T, N, Check>(checker: Check) -> PositiveLookahead<T, N, Check>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Check: HasId + IsCheckable<T> + 'static,
{
    PositiveLookahead::new(checker)
}

impl<T, N, Check> HasId for PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Check> IsCheckable<T> for PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Pure peek — pos must not move regardless of outcome.
        self.checker.check_no_advance(context, pos)
    }
}

impl<T, N, Check> Matcher<T> for PositiveLookahead<T, N, Check>
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
        if self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!("positive lookahead failed at position {}", pos))
        }
    }
}
