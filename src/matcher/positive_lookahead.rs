//! `&e` — succeeds when `checker` would match, without consuming input (position restored).

use crate::{
    error::error_handler::ErrorHandler,
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher},
};

/// Positive lookahead wrapper around a [`Matcher`].
#[derive(Clone, Debug)]
pub struct PositiveLookahead<Check> {
    checker: Check,
}

impl<Check> PositiveLookahead<Check> {
    /// See [`positive_lookahead`].
    pub fn new(checker: Check) -> Self {
        Self { checker }
    }
}

/// &e  — positive lookahead. Succeeds without consuming if `e` would match.
pub fn positive_lookahead<Check>(checker: Check) -> PositiveLookahead<Check> {
    PositiveLookahead::new(checker)
}

impl<Check> super::MatcherCombinator for PositiveLookahead<Check> where
    Check: super::MatcherCombinator
{
}

impl<'src, Inp: Input<'src>, MRes, Check> super::internal::MatcherImpl<'src, Inp, MRes>
    for PositiveLookahead<Check>
where
    Check: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = Check::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = Check::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let original_pos = input.get_pos();
        let result = runner.run_match(&self.checker, error_handler, input)?;
        input.set_pos(original_pos);
        Ok(result)
    }
}
