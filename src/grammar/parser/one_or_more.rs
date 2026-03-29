
use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    parser::Parser,
};
pub struct OneOrMoreParser<Pars, CombF> {
    parser: Pars,
    combine_fn: CombF,
    id: usize,
}

impl<Pars, CombF> OneOrMoreParser<Pars, CombF> {
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self {
            parser,
            combine_fn,
            id: get_next_id(),
        }
    }
}

impl<Pars, CombF> HasId for OneOrMoreParser<Pars, CombF> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Pars, CombF> IsCheckable<Token> for OneOrMoreParser<Pars, CombF>
where
    Pars: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        if !self.parser.check(context, pos) {
            return false;
        }
        while self.parser.check(context, pos) {}
        true
    }
}

impl<Token, NodeIn, NodeOut, Pars, CombF> Parser<Token> for OneOrMoreParser<Pars, CombF>
where
    Pars: Parser<Token, Output = NodeIn> + Grammar<Token>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;

    fn parse(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let mut results = Vec::new();
        // First match is mandatory — propagate the error if absent.
        results.push(self.parser.parse(context, pos)?);
        // Remaining matches are optional (same as Multiple).
        while self.parser.check_no_advance(context, pos) {
            results.push(self.parser.parse(context, pos)?);
        }
        Ok((self.combine_fn)(results))
    }
}
