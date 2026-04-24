use crate::matcher::internal::MatcherImpl;

#[derive(Debug, Clone)]
pub struct IfErrorMatcher<Inner> {
    inner: Inner,
    result_if_no_error: bool,
}

impl<Inner> super::MatcherCombinator for IfErrorMatcher<Inner> where Inner: super::MatcherCombinator {}

impl<Inner> IfErrorMatcher<Inner> {
    pub fn new(inner: Inner, result_if_no_error: bool) -> Self {
        Self {
            inner,
            result_if_no_error,
        }
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>, MRes>
    super::internal::MatcherImpl<'src, Inp, MRes> for IfErrorMatcher<Inner>
where
    Inner: MatcherImpl<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: super::MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if !error_handler.is_real() {
            return Ok(self.result_if_no_error);
        }
        self.inner.match_with_runner(runner, error_handler, input)
    }
}

pub fn if_error<Inner>(inner: Inner) -> IfErrorMatcher<Inner>
where
    Inner: super::MatcherCombinator,
{
    IfErrorMatcher {
        inner,
        result_if_no_error: true,
    }
}

pub fn if_error_else_fail<Inner>(inner: Inner) -> IfErrorMatcher<Inner>
where
    Inner: super::MatcherCombinator,
{
    IfErrorMatcher {
        inner,
        result_if_no_error: false,
    }
}
