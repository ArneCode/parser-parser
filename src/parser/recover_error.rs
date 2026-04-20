//! Error recovery: if the inner parser fails with [`crate::error::FurthestFailError`], try an alternate matcher.

use std::marker::PhantomData;

use crate::{
    context::ParserContext,
    error::error_handler::ErrorHandler,
    input::{InputFamily, InputStream},
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
impl<InpFam, Pars, Match, Output> super::internal::ParserImpl<InpFam>
    for ErrorRecoverer<Pars, Match, Output>
where
    InpFam: InputFamily + ?Sized,
    Pars: for<'src> Parser<InpFam, Output<'src> = Output>,
    Match: Matcher<InpFam, ((), (), ())>,
    Output: Clone,
{
    type Output<'src> = Output;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn parse<'src>(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, crate::error::FurthestFailError> {
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
