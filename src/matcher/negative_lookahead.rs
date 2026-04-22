//! `!e` — succeeds when `checker` does *not* match at the current position (no input consumed).

use crate::{
    error::error_handler::{EmptyErrorHandler, ErrorHandler},
    input::{Input, InputStream},
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

impl<'src, Inp: Input<'src>, MRes, Match> super::internal::MatcherImpl<'src, Inp, MRes>
    for NegativeLookahead<Match>
where
    Match: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;

    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;

    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, crate::error::FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let original_pos = input.get_pos();
        let mut inner_error_handler = EmptyErrorHandler::new(0);
        let can_match = runner.run_match(&self.checker, &mut inner_error_handler, input)?;
        input.set_pos(original_pos);
        Ok(!can_match)
    }
}
