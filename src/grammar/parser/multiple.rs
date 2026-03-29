
use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    parser::Parser,
};
pub struct MultipleParser<Pars, CombF> {
    parser: Pars,
    combine_fn: CombF,
    id: usize,
}

impl<Pars, CombF> MultipleParser<Pars, CombF> {
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self {
            parser,
            combine_fn,
            id: get_next_id(),
        }
    }
}

impl<Pars, CombF> HasId for MultipleParser<Pars, CombF> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, NodeIn, Pars, CombF> IsCheckable<T> for MultipleParser<Pars, CombF>
where
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<T, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        while self.parser.check(context, pos) {}
        true
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> Parser<T> for MultipleParser<Pars, CombF>
where
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;

    fn parse(
        &self,
        context: &mut ParserContext<T, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let mut results = Vec::new();
        while self.parser.check_no_advance(context, pos) {
            results.push(self.parser.parse(context, pos)?);
        }
        Ok((self.combine_fn)(results))
    }
}
