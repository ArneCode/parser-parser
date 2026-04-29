use crate::matcher::internal::MatcherImpl;

#[derive(Debug, Clone)]
pub struct IfError<Inner> {
    inner: Inner,
}

#[derive(Debug, Clone)]
pub struct IfErrorElseFail<Inner> {
    inner: Inner,
}

impl<Inner> super::MatcherCombinator for IfError<Inner> where Inner: super::MatcherCombinator {}
impl<Inner> super::MatcherCombinator for IfErrorElseFail<Inner> where Inner: super::MatcherCombinator {}

impl<Inner> crate::parser::ParserCombinator for IfErrorElseFail<Inner> where
    Inner: crate::parser::ParserCombinator
{
}

impl<Inner> IfError<Inner> {
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> IfErrorElseFail<Inner> {
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>, MRes>
    super::internal::MatcherImpl<'src, Inp, MRes> for IfError<Inner>
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
            return Ok(true);
        }
        self.inner.match_with_runner(runner, error_handler, input)
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>, MRes>
    super::internal::MatcherImpl<'src, Inp, MRes> for IfErrorElseFail<Inner>
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
            return Ok(false);
        }
        self.inner.match_with_runner(runner, error_handler, input)
    }
}

impl<'src, Inner, Inp: crate::input::Input<'src>> crate::parser::internal::ParserImpl<'src, Inp>
    for IfErrorElseFail<Inner>
where
    Inner: crate::parser::Parser<'src, Inp>,
{
    type Output = <Inner as crate::parser::internal::ParserImpl<'src, Inp>>::Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut crate::context::ParserContext,
        error_handler: &mut impl crate::error::error_handler::ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, crate::error::FurthestFailError> {
        if !error_handler.is_real() {
            return Ok(None);
        }
        self.inner.parse(context, error_handler, input)
    }
}

pub fn if_error<Inner>(inner: Inner) -> IfError<Inner>
where
    Inner: super::MatcherCombinator,
{
    IfError { inner }
}

pub fn if_error_else_fail<Inner>(inner: Inner) -> IfErrorElseFail<Inner>
{
    IfErrorElseFail { inner }
}
