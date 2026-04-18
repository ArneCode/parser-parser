use crate::{
    error::{FurthestFailError, MissingError, error_handler::ErrorHandler},
    matcher::{MatchRunner, Matcher},
};

pub struct InsertOnErrorMatcher<Inner> {
    pub inner: Inner,
    pub message: String,
}

impl<Token, MRes, Inner> Matcher<Token, MRes> for InsertOnErrorMatcher<Inner>
where
    Inner: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = true;

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
        let start_pos = *pos;
        match runner.run_match(&self.inner, error_handler, pos)? {
            true => Ok(true),
            false => {
                if error_handler.is_real() {
                    let error = MissingError {
                        message: self.message.clone(),
                        span: (start_pos, start_pos),
                    };
                    error_handler.register_stack_error(error.as_parser_error());
                    Ok(true) // We return true because we "inserted" the missing element
                } else {
                    Ok(false)
                }
            }
        }
    }
}
