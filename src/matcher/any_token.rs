//! Unconditional single-token consumption (if any input remains).

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::MatchRunner,
};

/// Matches exactly one token and advances; fails at end of input.
#[derive(Clone, Debug)]
pub struct AnyToken;

impl super::MatcherCombinator for AnyToken {}

impl<'src, Inp: Input<'src>, MRes> super::internal::MatcherImpl<'src, Inp, MRes> for AnyToken
where
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        Ok(input.next().is_some())
    }
}
