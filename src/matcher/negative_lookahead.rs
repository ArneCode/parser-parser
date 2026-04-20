//! `!e` — succeeds when `checker` does *not* match at the current position (no input consumed).

use crate::{
    error::error_handler::ErrorHandler,
    input::{InputFamily, InputStream},
    matcher::{MatchRunner, Matcher},
};

/// Predicate-style negative lookahead built from any [`Matcher`].
pub struct NegativeLookahead<Check> {
    checker: Check,
}

impl<Check> NegativeLookahead<Check> {
    /// Wraps `checker` (typically another matcher used as a probe).
    pub fn new(checker: Check) -> Self {
        Self { checker }
    }
}

/// Convenience constructor for [`NegativeLookahead`].
pub fn negative_lookahead<Check>(checker: Check) -> NegativeLookahead<Check> {
    NegativeLookahead::new(checker)
}

impl<InpFam, MRes, Match> super::internal::MatcherImpl<InpFam, MRes>
    for NegativeLookahead<Match>
where
    InpFam: InputFamily + ?Sized,
    Match: Matcher<InpFam, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;

    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;

    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        let original_pos = input.get_pos();
        let can_match = runner.run_match(&self.checker, error_handler, input)?;
        input.set_pos(original_pos);
        Ok(!can_match)
    }
}
