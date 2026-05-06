//! Greedy repetition: parse zero or more `Pars` outputs, then fold with `combine_fn`.

use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::{Parser, ParserCombinator},
};

/// Applies `parser` in a loop until it returns [`None`], then maps the collected vector with `combine_fn`.
#[derive(Clone)]
pub struct MultipleParser<Pars, CombF> {
    parser: Pars,
    combine_fn: CombF,
}

impl<Pars, CombF> std::fmt::Debug for MultipleParser<Pars, CombF> where
    Pars: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultipleParser")
            .field("parser", &self.parser)
            .finish()
    }
}

impl<Pars, CombF> MultipleParser<Pars, CombF> {
    /// Creates a greedy “many” parser wrapper.
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self { parser, combine_fn }
    }
}

impl<Pars, CombF> ParserCombinator for MultipleParser<Pars, CombF> where
    Pars: ParserCombinator
{
}

impl<'src, Inp: Input<'src>, NodeIn, NodeOut, Pars, CombF> super::internal::ParserImpl<'src, Inp>
    for MultipleParser<Pars, CombF>
where
    Pars: Parser<'src, Inp, Output = NodeIn> + Clone,
    Inp: Input<'src>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut + Clone,
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

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.parser.maybe_label()
    }
}
