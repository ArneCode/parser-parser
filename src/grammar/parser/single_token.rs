use std::fmt::Debug;

use crate::grammar::{
    HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    label::MaybeLabel, parser::Parser,
};

pub struct SingleTokenParser<Token> {
    token: Token,
    id: usize,
}

impl<Token> SingleTokenParser<Token> {
    pub fn new(token: Token) -> Self {
        Self {
            token,
            id: get_next_id(),
        }
    }
}

impl<Token> HasId for SingleTokenParser<Token> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token: PartialEq> IsCheckable<Token> for SingleTokenParser<Token> {
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        if *pos < context.tokens.len() {
            let token = &context.tokens[*pos];
            if token == &self.token {
                *pos += 1; // Advance position on success
                return true;
            }
        }
        false
    }
}

impl<Token: PartialEq + Clone + Debug> Parser<Token> for SingleTokenParser<Token> {
    type Output = Token;

    fn parse(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        if self.calc_check(context, pos) {
            Ok(self.token.clone())
        } else {
            Err(format!(
                "Expected token {:?} at position {}, but found {:?}",
                self.token,
                *pos,
                context.tokens.get(*pos)
            ))
        }
    }
}

impl<Token, Label> MaybeLabel<Label> for SingleTokenParser<Token> {}
