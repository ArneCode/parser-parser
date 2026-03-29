use crate::grammar::{
    HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
};
use std::{error::Error, marker::PhantomData, ops::Deref};
pub struct AnyToken {
    id: usize,
}

impl Default for AnyToken {
    fn default() -> Self {
        Self::new()
    }
}

impl AnyToken {
    pub fn new() -> Self {
        Self { id: get_next_id() }
    }
}

/// `.`  — match any single token without inspecting its value.
pub fn any_token() -> AnyToken {
    AnyToken::new()
}

impl HasId for AnyToken {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token> IsCheckable<Token> for AnyToken {
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        if *pos < context.tokens.len() {
            *pos += 1;
            true
        } else {
            false
        }
    }
}

impl<Token, MRes> Matcher<Token, MRes> for AnyToken {
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if *pos < context.parser_context.tokens.len() {
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
