use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};

pub struct ParserMatcher<Pars, ParserOutput> {
    parser: Pars,
    expected_output: ParserOutput,
}

impl<Pars, ParserOutput> ParserMatcher<Pars, ParserOutput> {
    pub fn new(parser: Pars, expected_output: ParserOutput) -> Self {
        Self {
            parser,
            expected_output,
        }
    }
}

impl<Token, MRes, Pars, ParserOutput> Matcher<Token, MRes> for ParserMatcher<Pars, ParserOutput>
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
    ) -> Result<bool, ParserError>
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
