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
impl<Token, MRes, Match, Pars, F> Matcher<Token, MRes> for ErrorContextualizer<Match, Pars, F>
where
    Match: Matcher<Token, MRes>,
    Pars: Parser<Token, Output = F>,
    F: Fn(&mut ParserError),
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = Match::CAN_FAIL;

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
        match runner.run_match(&self.happy_matcher, error_handler, pos) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(mut e) => {
                let mut start_pos = *pos;
                if let Ok(Some(f)) = self.error_parser.parse(
                    runner.get_parser_context(),
                    error_handler,
                    &mut start_pos,
                ) {
                    f(&mut e);
                }
                Err(e)
            }
        }
    }
}
