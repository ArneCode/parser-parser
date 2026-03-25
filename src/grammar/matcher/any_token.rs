use crate::grammar::{
    AstNode, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N> AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    pub fn new() -> Self {
        Self {
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// `.`  — match any single token without inspecting its value.
pub fn any_token<T, N>() -> AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    AnyToken::new()
}

impl<T, N> HasId for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N> IsCheckable<T> for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if *pos < context.tokens.len() {
            *pos += 1;
            true
        } else {
            false
        }
    }
}

impl<T, N> Matcher<T> for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if *pos < context.tokens.len() {
            *pos += 1;
            Ok(())
        } else {
            Err(format!(
                "expected any token at position {} but reached end of input",
                pos
            ))
        }
    }
}
