//! Error recovery: if the inner parser fails with [`crate::error::FurthestFailError`], try an alternate matcher.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    context::ParserContext,
    error::error_handler::ErrorHandler,
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, MatcherCombinator, NoMemoizeBacktrackingRunner},
    parser::{Parser, ParserCombinator},
};

static NEXT_RECOVER_ID: AtomicUsize = AtomicUsize::new(0);

/// On hard failure of `happy`, resets position and runs `recover_matcher`; on success yields `recover_output` and records the error.
#[derive(Clone)]
pub struct ErrorRecoverer<HappyParser, RecoveryParser> {
    happy: HappyParser,
    recover_parser: RecoveryParser,
    id: usize,
}

impl<Pars, RecoveryParser> std::fmt::Debug for ErrorRecoverer<Pars, RecoveryParser>
where
    Pars: std::fmt::Debug,
    RecoveryParser: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorRecoverer")
            .field("happy", &self.happy)
            .field("recover_parser", &self.recover_parser)
            .finish()
    }
}

impl<HappyParser, RecoveryParser> ErrorRecoverer<HappyParser, RecoveryParser> {
    /// See [`crate::parser::Parser::recover_with`].
    pub fn new(happy: HappyParser, recover_parser: RecoveryParser) -> Self {
        Self {
            happy,
            recover_parser,
            id: NEXT_RECOVER_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<HappyParser, RecoveryParser> ParserCombinator for ErrorRecoverer<HappyParser, RecoveryParser>
where
    HappyParser: ParserCombinator,
    RecoveryParser: ParserCombinator,
{
}

//TODO: ensure that Match cannot error with trait CanNotError
impl<'src, Inp: Input<'src>, HappyParser, RecoveryParser> super::internal::ParserImpl<'src, Inp>
    for ErrorRecoverer<HappyParser, RecoveryParser>
where
    HappyParser: Parser<'src, Inp>,
    RecoveryParser: Parser<'src, Inp, Output = HappyParser::Output>,
    Inp: Input<'src>,
{
    type Output = HappyParser::Output;
    const CAN_FAIL: bool = HappyParser::CAN_FAIL;

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
                // let mut runner = NoMemoizeBacktrackingRunner::new(context);
                // if runner
                //     .run_match(&self.recover_matcher, error_handler, input)
                //     .unwrap_or(false)
                if let Some(output) = self
                    .recover_parser
                    .parse(context, error_handler, input)
                    .unwrap_or(None)
                {
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

                    return Ok(Some(output));
                }
                Err(e)
            }
            Ok(output) => Ok(output),
        }
    }
}
