use std::{marker::PhantomData, rc::Rc};

use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, get_next_id, parser::Parser,
};
pub struct TokenParser<T, N, CheckF, ParseF>
where
// CheckF: Fn(&T) -> bool,
// ParseF: Fn(&T) -> Box<N>,
{
    check_fn: CheckF,
    parse_fn: ParseF,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, CheckF, ParseF> TokenParser<T, N, CheckF, ParseF>
where
// CheckF: Fn(&T) -> bool,
// ParseF: Fn(&T) -> Box<N>,
{
    pub fn new(check_fn: CheckF, parse_fn: ParseF) -> Self {
        Self {
            check_fn,
            parse_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, N, CheckF, ParseF> HasId for TokenParser<T, N, CheckF, ParseF> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, CheckF, ParseF> IsCheckable<T> for TokenParser<T, N, CheckF, ParseF>
where
    CheckF: Fn(&T) -> bool,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if *pos < context.tokens.len() {
            let token = &context.tokens[*pos];
            *pos += 1; // Advance position on success
            (self.check_fn)(token)
        } else {
            false
        }
    }
}

impl<T, N, CheckF, ParseF> Parser<T> for TokenParser<T, N, CheckF, ParseF>
where
    CheckF: Fn(&T) -> bool,
    ParseF: Fn(&T) -> N,
{
    type Output = N;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        if *pos < context.tokens.len() {
            if self.check_no_advance(&context, pos) {
                let token = &context.tokens[*pos];
                *pos += 1; // Advance position on success
                Ok((self.parse_fn)(token))
            } else {
                Err(format!(
                    "token did not satisfy check function at position {}",
                    pos
                ))
            }
        } else {
            Err(format!(
                "expected token at position {} but reached end of input",
                pos
            ))
        }
    }
}
