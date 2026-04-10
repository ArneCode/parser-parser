
use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{CanImplMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner, MatchRunner},
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

impl<'a, 'ctx, Pars, ParserOutput, Runner> CanImplMatchWithRunner<Runner>
    for ParserMatcher<Pars, ParserOutput>
where
    Runner: MatchRunner<'a, 'ctx>,
    Pars: Parser<'ctx, Runner::Token, Output = ParserOutput>,
    ParserOutput: PartialEq,
{
    fn impl_match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
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
impl<Pars, ParserOutput> DoImplMatchWithNoMoemoizeBacktrackingRunner
    for ParserMatcher<Pars, ParserOutput>
{
}
