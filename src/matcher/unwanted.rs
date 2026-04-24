use crate::{
    error::{FurthestFailError, UnwantedError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatcherCombinator, internal::MatcherImpl},
};

#[derive(Clone, Debug)]
pub struct UnwantedMatcher<Inner> {
    inner: Inner,
    message: String,
}

impl<Inner> UnwantedMatcher<Inner> {
    pub fn new(inner: Inner, message: String) -> Self {
        Self { inner, message }
    }
}

impl<Inner> MatcherCombinator for UnwantedMatcher<Inner> where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner> MatcherImpl<'src, Inp, MRes> for UnwantedMatcher<Inner>
where
    Inner: MatcherImpl<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let start_pos: usize = input.get_pos().into();
        if runner.run_match(&self.inner, error_handler, input)? {
            if error_handler.is_real() {
                let end_pos: usize = input.get_pos().into();
                let error = UnwantedError {
                    span: (start_pos, end_pos),
                    message: self.message.clone(),
                };
                runner.get_parser_context().push_stack_error(error.as_parser_error());
                return Ok(true); // We return true because we "inserted" the unwanted element
            }
            return Ok(false);
        }
        Ok(true)
    }
}

pub fn unwanted<Inner>(inner: Inner, message: impl Into<String>) -> UnwantedMatcher<Inner>
where
    Inner: MatcherCombinator,
{
    UnwantedMatcher {
        inner,
        message: message.into(),
    }
}
