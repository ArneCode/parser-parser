use std::marker::PhantomData;

use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};

pub struct ErrorContextualizer<Matcher, Pars, F> {
    happy_matcher: Matcher,
    error_parser: Pars,
    _phantom: PhantomData<F>,
}

impl<Matcher, Pars, F> ErrorContextualizer<Matcher, Pars, F> {
    pub fn new(happy_matcher: Matcher, error_parser: Pars) -> Self {
        Self {
            happy_matcher,
            error_parser,
            _phantom: PhantomData,
        }
    }
}

//TODO: ensure that Pars cannot error with trait CanNotError
impl<'a, 'ctx, Match, Pars, F, Runner> Matcher<Runner> for ErrorContextualizer<Match, Pars, F>
where
    Runner: MatchRunner<'a, 'ctx>,
    Match: Matcher<Runner>,
    Pars: Parser<Runner::Token, Output = F>,
    F: Fn(&mut ParserError) -> (),
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
        match runner.run_match(&self.happy_matcher, error_handler, pos) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(mut e) => {
                let mut start_pos = *pos;
                match self.error_parser.parse(
                    runner.get_parser_context(),
                    error_handler,
                    &mut start_pos,
                ) {
                    Ok(Some(f)) => {
                        f(&mut e);
                    }
                    _ => {}
                }
                Err(e)
            }
        }
    }
}
