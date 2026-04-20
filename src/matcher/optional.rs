//! Optional matcher: tries `matcher` once; always reports success (whether or not it matched).

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
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

impl<InpFam, MRes, Match> super::internal::MatcherImpl<InpFam, MRes> for Optional<Match>
where
    InpFam: InputFamily + ?Sized,
    Match: Matcher<InpFam, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = false;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        if runner.run_match(&self.matcher, error_handler, input)? {
            return Ok(true);
        }
        Ok(true)
    }
}
