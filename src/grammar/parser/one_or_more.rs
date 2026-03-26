use std::{marker::PhantomData, rc::Rc};

use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, get_next_id, parser::Parser,
};
pub struct OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    parser: Pars,
    combine_fn: CombF,
    id: usize,
    _phantom: PhantomData<(T, NodeIn, NodeOut)>,
}

impl<T, NodeIn, NodeOut, Pars, CombF> OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    Pars: Parser<T, Output = NodeIn>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self {
            parser,
            combine_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> HasId for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> IsCheckable<T>
    for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if !self.parser.check(context, pos) {
            return false;
        }
        while self.parser.check(context, pos) {}
        true
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> Parser<T> for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let mut results = Vec::new();
        // First match is mandatory — propagate the error if absent.
        results.push(self.parser.parse(context.clone(), pos)?);
        // Remaining matches are optional (same as Multiple).
        while self.parser.check_no_advance(&context, pos) {
            results.push(self.parser.parse(context.clone(), pos)?);
        }
        Ok((self.combine_fn)(results))
    }
}
