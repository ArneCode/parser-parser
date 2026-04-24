use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::{Parser, ParserCombinator},
};

#[derive(Clone)]
pub struct OutputMapper<Parser, MapFn> {
    parser: Parser,
    map_fn: MapFn,
}

impl<Parser, MapFn> std::fmt::Debug for OutputMapper<Parser, MapFn>
where
    Parser: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputMapper")
            .field("parser", &self.parser)
            .finish()
    }
}

impl<Parser, MapFn> OutputMapper<Parser, MapFn> {
    pub fn new(parser: Parser, map_fn: MapFn) -> Self {
        Self { parser, map_fn }
    }
}

impl<Parser, MapFn> ParserCombinator for OutputMapper<Parser, MapFn> where Parser: ParserCombinator {}

impl<'src, Inp: Input<'src>, NodeIn, NodeOut, Pars, MapFn> super::internal::ParserImpl<'src, Inp>
    for OutputMapper<Pars, MapFn>
where
    Pars: Parser<'src, Inp, Output = NodeIn> + Clone,
    Inp: Input<'src>,
    MapFn: Fn(NodeIn) -> NodeOut + Clone,
{
    type Output = NodeOut;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        if let Some(result) = self.parser.parse(context, error_handler, input)? {
            Ok(Some((self.map_fn)(result)))
        } else {
            Ok(None)
        }
    }
}
