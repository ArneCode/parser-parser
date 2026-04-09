use std::fmt::Debug;

use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::{
        CanImplMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner, MatchRunner, Matcher,
    },
    parser::Parser,
};

pub struct ParserMatcher<Pars, ParserOutput> {
    parser: Pars,
    expected_output: ParserOutput,
    id: usize,
}

impl<Pars, ParserOutput> ParserMatcher<Pars, ParserOutput> {
    pub fn new(parser: Pars, expected_output: ParserOutput) -> Self {
        Self {
            parser,
            expected_output,
            id: get_next_id(),
        }
    }
}

impl<Pars, ParserOutput> HasId for ParserMatcher<Pars, ParserOutput> {
    fn id(&self) -> usize {
        self.id
    }
}

impl<Token, Pars, ParserOutput> IsCheckable<Token> for ParserMatcher<Pars, ParserOutput>
where
    Pars: Parser<Token, Output = ParserOutput> + Grammar<Token>,
    ParserOutput: PartialEq,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        if let Ok(output) = self.parser.parse(context, pos)
            && output == self.expected_output
        {
            return true;
        }
        false
    }
}

impl<Token, MRes, Pars, ParserOutput> Matcher<Token, MRes> for ParserMatcher<Pars, ParserOutput>
where
    Pars: Parser<Token, Output = ParserOutput> + Grammar<Token>,
    ParserOutput: PartialEq,
    ParserOutput: Debug,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.calc_check(context.parser_context, pos) {
            Ok(())
        } else {
            Err(format!(
                "Expected parser output {:?} at position {}",
                self.expected_output, pos
            ))
        }
    }
}

impl<'a, 'ctx, Pars, ParserOutput, Runner> CanImplMatchWithRunner<Runner>
    for ParserMatcher<Pars, ParserOutput>
where
    Runner: MatchRunner<'a, 'ctx>,
    Pars: Parser<Runner::Token, Output = ParserOutput> + Grammar<Runner::Token>,
    ParserOutput: PartialEq,
{
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String> {
        if let Ok(output) = self.parser.parse(runner.get_parser_context(), pos)
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

impl<Pars, ParserOutput> MaybeLabel<String> for ParserMatcher<Pars, ParserOutput>
where
    ParserOutput: Debug,
{
    fn maybe_label(&self) -> Option<String> {
        Some(format!("{:?}", self.expected_output))
    }
}
