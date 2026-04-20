//! Error recovery: if the inner parser fails with [`crate::error::FurthestFailError`], try an alternate matcher.

use std::marker::PhantomData;

use crate::{
    context::ParserContext,
    error::error_handler::ErrorHandler,
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, NoMemoizeBacktrackingRunner},
    parser::Parser,
};

/// On hard failure of `happy`, resets position and runs `recover_matcher`; on success yields `recover_output` and records the error.
pub struct ErrorRecoverer<Pars, Match, Output> {
    happy: Pars,
    recover_matcher: Match,
    recover_output: Output,
}

impl<Pars, Match, Output> ErrorRecoverer<Pars, Match, Output> {
    /// See [`crate::parser::Parser::recover_with`].
    pub fn new(happy: Pars, recover_matcher: Match, recover_output: Output) -> Self {
        Self {
            happy,
            recover_matcher,
            recover_output,
        }
    }
}

//TODO: ensure that Match cannot error with trait CanNotError
impl<'src, Inp: Input<'src>, Pars, Match, Output> super::internal::ParserImpl<'src, Inp>
    for ErrorRecoverer<Pars, Match, Output>
where
    Pars: Parser<'src, Inp, Output = Output>,
    Match: Matcher<'src, Inp, ((), (), ())>,
    Inp: Input<'src>,
    Output: Clone,
{
    type Output = Output;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, crate::error::FurthestFailError> {
        let start_pos = input.get_pos();
        match self.happy.parse(context, error_handler, input) {
            Err(e) => {
                input.set_pos(start_pos);
                let mut runner = NoMemoizeBacktrackingRunner::new(context, PhantomData::<&'src ()>);
                if runner
                    .run_match(&self.recover_matcher, error_handler, input)
                    .unwrap_or(false)
                {
                    drop(runner);
                    context.error_sink.push(e.as_parser_error());
                    return Ok(Some(self.recover_output.clone()));
                }
                Err(e)
            }
            Ok(output) => Ok(output),
        }
    }
}
