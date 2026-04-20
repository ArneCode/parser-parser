//! Enrich [`crate::error::FurthestFailError`] from `happy_matcher` using a small [`crate::parser::Parser`] callback.

use std::marker::PhantomData;

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};

/// On [`Err`] from the inner matcher, runs `error_parser` to obtain a callback that mutates the error.
pub struct ErrorContextualizer<Matcher, Pars, F, MRes> {
    happy_matcher: Matcher,
    error_parser: Pars,
    _phantom: PhantomData<(MRes, F)>,
}

impl<Matcher, Pars, F, MRes> ErrorContextualizer<Matcher, Pars, F, MRes> {
    /// See [`crate::matcher::Matcher::add_error_info`].
    pub fn new(happy_matcher: Matcher, error_parser: Pars) -> Self {
        Self {
            happy_matcher,
            error_parser,
            _phantom: PhantomData,
        }
    }
}

//TODO: ensure that Pars cannot error with trait CanNotError
impl<InpFam, MRes, Match, Pars, F> super::internal::MatcherImpl<InpFam, MRes>
    for ErrorContextualizer<Match, Pars, F, MRes>
where
    InpFam: InputFamily + ?Sized,
    Match: Matcher<InpFam, MRes>,
    Pars: for<'src> Parser<InpFam, Output<'src> = F>,
    F: Fn(&mut FurthestFailError),
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        let start_pos = input.get_pos();
        match runner.run_match(&self.happy_matcher, error_handler, input) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(mut e) => {
                let resume_pos = input.get_pos();
                input.set_pos(start_pos.clone());
                if let Ok(Some(f)) = self.error_parser.parse(
                    runner.get_parser_context(),
                    error_handler,
                    input,
                )
                {
                    f(&mut e);
                }
                input.set_pos(resume_pos);
                Err(e)
            }
        }
    }
}
