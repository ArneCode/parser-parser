//! Enrich [`crate::error::FurthestFailError`] from `happy_matcher` using a small [`crate::parser::Parser`] callback.

use std::marker::PhantomData;

use crate::{
    context::ParserContext, error::{FurthestFailError, error_handler::ErrorHandler}, input::{Input, InputStream}, matcher::{MatchRunner, Matcher, MatcherCombinator}, parser::{Parser, ParserCombinator, internal::ParserImpl}
};

/// On [`Err`] from the inner matcher, runs `error_parser` to obtain a callback that mutates the error.
pub struct ErrorContextualizer<Happy, Pars> {
    happy: Happy,
    error_parser: Pars,
}

impl<Matcher, Pars> MatcherCombinator for ErrorContextualizer<Matcher, Pars> where Matcher: MatcherCombinator {}
impl<Happy, Pars> ParserCombinator for ErrorContextualizer<Happy, Pars> where Happy: ParserCombinator {}

impl<Matcher, Pars> ErrorContextualizer<Matcher, Pars> {
    /// See [`crate::matcher::Matcher::add_error_info`].
    pub fn new(happy: Matcher, error_parser: Pars) -> Self {
        Self {
            happy: happy,
            error_parser,
        }
    }
}

//TODO: ensure that Pars cannot error with trait CanNotError
impl<'src, Inp: Input<'src>, Happy, Pars, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for ErrorContextualizer<Happy, Pars>
where
    Happy: Matcher<'src, Inp, MRes>,
    Pars: Parser<'src, Inp, Output = Box<dyn Fn(&mut FurthestFailError)>>,
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = Happy::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Happy::HAS_PROPERTY;
    const CAN_FAIL: bool = Happy::CAN_FAIL;

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
        let start_pos = input.get_pos();
        match runner.run_match(&self.happy, error_handler, input) {
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

// impl Parser
impl<'src, Inp: Input<'src>, Happy, Pars> ParserImpl<'src, Inp> for ErrorContextualizer<Happy, Pars> 
where 
    Happy: Parser<'src, Inp>, 
    Pars: Parser<'src, Inp, Output = Box<dyn Fn(&mut FurthestFailError)>>,
    Inp: Input<'src>,
{
    type Output = Happy::Output;
    const CAN_FAIL: bool = Happy::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start_pos = input.get_pos();
        match self.happy.parse(context, error_handler, input) {
            Ok(Some(output)) => Ok(Some(output)),
            Ok(None) => Ok(None),
            Err(mut e) => {
                let resume_pos = input.get_pos();
                input.set_pos(start_pos);
                if let Ok(Some(f)) = self.error_parser.parse(
                    context,
                    error_handler,
                    input,
                )
                {
                    f(&mut e);
                }
                input.set_pos(resume_pos);
                Err(e)
            },
        }
    }
}