use crate::grammar::{
    context::ParserContext,
    error_handler::ErrorHandler,
    matcher::{CanMatchWithRunner, MatchRunner, NoMoemoizeBacktrackingRunner},
    parser::Parser,
};

pub struct ErrorRecoverer<Pars, Match, Output> {
    happy: Pars,
    recover_matcher: Match,
    recover_output: Output,
}

impl<Pars, Match, Output> ErrorRecoverer<Pars, Match, Output> {
    pub fn new(happy: Pars, recover_matcher: Match, recover_output: Output) -> Self {
        Self {
            happy,
            recover_matcher,
            recover_output,
        }
    }
}

//TODO: ensure that Match cannot error with trait CanNotError
impl<'ctx, Pars, Match, Output, Token> Parser<'ctx, Token> for ErrorRecoverer<Pars, Match, Output>
where
    Pars: Parser<'ctx, Token, Output = Output>,
    Token: 'ctx,
    Match: for<'a> CanMatchWithRunner<NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, ((), (), ())>>,
    Output: Clone,
{
    type Output = Output;

    fn parse(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, crate::grammar::error_handler::ParserError> {
        let start_pos = *pos;
        match self.happy.parse(context, error_handler, pos) {
            Err(e) => {
                *pos = start_pos;
                let mut runner = NoMoemoizeBacktrackingRunner::new(context);
                if runner
                    .run_match(&self.recover_matcher, error_handler, pos)
                    .unwrap_or(false)
                {
                    drop(runner);
                    context.error_sink.push(e);
                    return Ok(Some(self.recover_output.clone()));
                }
                return Err(e);
            }
            Ok(output) => Ok(output),
        }
    }
}
