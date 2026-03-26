use crate::grammar::{
    HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};
pub struct AnyToken<T> {
    id: usize,
    _phantom: PhantomData<(T)>,
}

impl<T> AnyToken<T> {
    pub fn new() -> Self {
        Self {
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// `.`  — match any single token without inspecting its value.
pub fn any_token<T>() -> AnyToken<T> {
    AnyToken::new()
}

impl<T> HasId for AnyToken<T> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<T> IsCheckable<T> for AnyToken<T> {
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if *pos < context.tokens.len() {
            *pos += 1;
            true
        } else {
            false
        }
    }
}

impl<T, MContext> Matcher<T, MContext> for AnyToken<T>
where
    MContext: Deref<Target = ParserContext<T>>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
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
