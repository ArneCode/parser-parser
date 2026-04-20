//! Optional matcher: tries `matcher` once; always reports success (whether or not it matched).

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher},
};

/// `matcher?` at the matcher level.
pub struct Optional<Match> {
    matcher: Match,
}

impl<Match> Optional<Match> {
    fn new(matcher: Match) -> Self {
        Self { matcher }
    }
}

/// See [`Optional`].
pub fn optional<Match>(matcher: Match) -> Optional<Match> {
    Optional::new(matcher)
}

impl<'src, Inp: Input<'src>, MRes, Match> super::internal::MatcherImpl<'src, Inp, MRes> for Optional<Match>
where
    Match: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = false;

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
        if runner.run_match(&self.matcher, error_handler, input)? {
            return Ok(true);
        }
        Ok(true)
    }
}
