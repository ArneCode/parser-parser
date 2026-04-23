//! When the inner matcher fails “softly” and the error handler is active, record a synthetic [`crate::error::MissingError`].

use crate::{
    error::{FurthestFailError, MissingError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher},
};

/// Wrapper produced by [`crate::matcher::Matcher::try_insert_if_missing`].
pub struct InsertOnErrorMatcher<Inner> {
    /// Inner matcher.
    pub inner: Inner,
    /// Message stored on the synthetic missing error.
    pub message: String,
}

impl<'src, Inp: Input<'src>, MRes, Inner> super::internal::MatcherImpl<'src, Inp, MRes>
    for InsertOnErrorMatcher<Inner>
where
    Inner: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = true;

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
        let start_pos: usize = input.get_pos().into();
        match runner.run_match(&self.inner, error_handler, input)? {
            true => Ok(true),
            false => {
                if error_handler.is_real() {
                    let error = MissingError {
                        message: self.message.clone(),
                        span: (start_pos, start_pos),
                    };
                    runner.get_parser_context().push_stack_error(error.as_parser_error());
                    Ok(true) // We return true because we "inserted" the missing element
                } else {
                    Ok(false)
                }
            }
        }
    }
}
