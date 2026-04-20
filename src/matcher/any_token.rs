//! Unconditional single-token consumption (if any input remains).

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::MatchRunner,
};

/// Matches exactly one token and advances; fails at end of input.
pub struct AnyToken;

impl<InpFam, MRes> super::internal::MatcherImpl<InpFam, MRes> for AnyToken
where
    InpFam: InputFamily + ?Sized,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        Ok(input.next().is_some())
    }
}
