//! Greedy repetition: parse zero or more `Pars` outputs, then fold with `combine_fn`.

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::Parser,
};

/// Applies `parser` in a loop until it returns [`None`], then maps the collected vector with `combine_fn`.
pub struct MultipleParser<Pars, CombF> {
    parser: Pars,
    combine_fn: CombF,
}

impl<Pars, CombF> MultipleParser<Pars, CombF> {
    /// Creates a greedy “many” parser wrapper.
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

impl<'src, Inp: Input<'src>, NodeIn, NodeOut, Pars, CombF> super::internal::ParserImpl<'src, Inp>
    for MultipleParser<Pars, CombF>
where
    Pars: Parser<'src, Inp, Output = NodeIn>,
    Inp: Input<'src>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError>
    where
        Inp: Input<'src>,
    {
        let mut results = Vec::new();
        while let Some(result) = self.parser.parse(context, error_handler, input)? {
            results.push(result);
        }
        Ok(Some((self.combine_fn)(results)))
    }
}
