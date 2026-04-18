//! Treat a [`crate::parser::Parser`] as a [`crate::matcher::Matcher`]: succeed only when parse output equals `expected_output`.

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::MatchRunner,
    parser::Parser,
};

/// Runs `parser` and compares the result to `expected_output` with [`PartialEq`].
pub struct ParserMatcher<Pars, ParserOutput> {
    parser: Pars,
    expected_output: ParserOutput,
}

impl<Pars, ParserOutput> ParserMatcher<Pars, ParserOutput> {
    /// Matcher succeeds when `parser` returns `Some(expected_output)`.
    pub fn new(parser: Pars, expected_output: ParserOutput) -> Self {
        Self {
            parser,
            expected_output,
        }
    }
}

impl<Token, MRes, Pars, ParserOutput> super::internal::MatcherImpl<Token, MRes>
    for ParserMatcher<Pars, ParserOutput>
where
    Pars: Parser<Token, Output = ParserOutput>,
    ParserOutput: PartialEq,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        if let Some(output) = self
            .parser
            .parse(runner.get_parser_context(), error_handler, pos)?
            && output == self.expected_output
        {
            return Ok(true);
        }
        Ok(false)
    }
}
