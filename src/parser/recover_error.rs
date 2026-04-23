//! Error recovery: if the inner parser fails with [`crate::error::FurthestFailError`], try an alternate matcher.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    context::ParserContext,
    error::error_handler::ErrorHandler,
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, NoMemoizeBacktrackingRunner},
    parser::Parser,
};

static NEXT_RECOVER_ID: AtomicUsize = AtomicUsize::new(0);

/// On hard failure of `happy`, resets position and runs `recover_matcher`; on success yields `recover_output` and records the error.
pub struct ErrorRecoverer<Pars, Match, Output> {
    happy: Pars,
    recover_matcher: Match,
    recover_output: Output,
    id: usize,
}

impl<Pars, Match, Output> ErrorRecoverer<Pars, Match, Output> {
    /// See [`crate::parser::Parser::recover_with`].
    pub fn new(happy: Pars, recover_matcher: Match, recover_output: Output) -> Self {
        Self {
            happy,
            recover_matcher,
            recover_output,
            id: NEXT_RECOVER_ID.fetch_add(1, Ordering::Relaxed),
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
                input.set_pos(start_pos.clone());
                let mut runner = NoMemoizeBacktrackingRunner::new(context);
                if runner
                    .run_match(&self.recover_matcher, error_handler, input)
                    .unwrap_or(false)
                {
                    drop(runner);
                    // TODO: maybe find a way to avoid registering the same error multiple times.
                    if !context
                        .registered_error_set
                        .contains(&(self.id, start_pos.clone().into()))
                    {
                        context.error_sink.push(e.as_parser_error());
                        context
                            .registered_error_set
                            .insert((self.id, start_pos.into()));
                    }

                    return Ok(Some(self.recover_output.clone()));
                }
                Err(e)
            }
            Ok(output) => Ok(output),
        }
    }
}
