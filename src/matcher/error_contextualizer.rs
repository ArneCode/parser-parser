use std::marker::PhantomData;

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};

pub struct ErrorContextualizer<Matcher, Pars, F, MRes> {
    happy_matcher: Matcher,
    error_parser: Pars,
    _phantom: PhantomData<(MRes, F)>,
}

impl<Matcher, Pars, F, MRes> ErrorContextualizer<Matcher, Pars, F, MRes> {
    pub fn new(happy_matcher: Matcher, error_parser: Pars) -> Self {
        Self {
            happy_matcher,
            error_parser,
            _phantom: PhantomData,
        }
    }
}

//TODO: ensure that Pars cannot error with trait CanNotError
impl<Token, MRes, Match, Pars, F> Matcher<Token, MRes> for ErrorContextualizer<Match, Pars, F, MRes>
where
    Match: Matcher<Token, MRes>,
    Pars: Parser<Token, Output = F>,
    F: Fn(&mut FurthestFailError),
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = Match::CAN_FAIL;

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
        let mut start_pos = *pos;
        match runner.run_match(&self.happy_matcher, error_handler, pos) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(mut e) => {
                // let mut start_pos = *pos;
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
