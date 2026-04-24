use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, MatcherCombinator},
    parser::{Parser, ParserCombinator},
};
#[derive(Clone, Debug)]
pub struct IgnoreResult<Parser> {
    parser: Parser,
}

impl<Parser> MatcherCombinator for IgnoreResult<Parser> where Parser: ParserCombinator {}

impl<Parser> IgnoreResult<Parser> {
    pub fn new(parser: Parser) -> Self {
        Self { parser }
    }
}

impl<'src, Inp: Input<'src>, MRes, Pars> super::internal::MatcherImpl<'src, Inp, MRes>
    for IgnoreResult<Pars>
where
    Pars: Parser<'src, Inp>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let result = self
            .parser
            .parse(runner.get_parser_context(), error_handler, input)?;
        Ok(result.is_some())
    }
}
