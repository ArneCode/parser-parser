use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    parser::Parser,
};
pub struct MultipleParser<Pars, CombF> {
    parser: Pars,
    combine_fn: CombF,
}

impl<Pars, CombF> MultipleParser<Pars, CombF> {
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self { parser, combine_fn }
    }
}

// impl<Pars, CombF> HasId for MultipleParser<Pars, CombF> {
//     fn id(&self) -> usize {
//         self.id
//     }
// }

// impl<T, NodeIn, Pars, CombF> IsCheckable<T> for MultipleParser<Pars, CombF>
// where
//     Pars: Parser<T, Output = NodeIn> + Grammar<T>,
// {
//     fn calc_check(&self, context: &mut ParserContext<T>, pos: &mut usize) -> bool {
//         while self.parser.check(context, pos) {}
//         true
//     }
// }

impl<'ctx, T, NodeIn, NodeOut, Pars, CombF> Parser<'ctx, T> for MultipleParser<Pars, CombF>
where
    Pars: Parser<'ctx, T, Output = NodeIn>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext<'ctx, T>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        let mut results = Vec::new();
        while let Some(result) = self.parser.parse(context, error_handler, pos)? {
            results.push(result);
        }
        Ok(Some((self.combine_fn)(results)))
    }
}
