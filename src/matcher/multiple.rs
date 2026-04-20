//! Zero-or-more repetition matcher; stops when `matcher` fails or makes no progress.

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::{MatchRunner, Matcher},
};

/// Greedy `matcher*` at the matcher level (always reports match success after the loop).
pub struct Multiple<Match> {
    matcher: Match,
}

impl<Match> Multiple<Match> {
    fn new(matcher: Match) -> Self {
        Self { matcher }
    }
}

/// See [`Multiple`].
pub fn many<Match>(matcher: Match) -> Multiple<Match> {
    Multiple::new(matcher)
}

// impl<Match> Matcher for Multiple<Match> where Match: Matcher {}

impl<InpFam, MRes, Match> super::internal::MatcherImpl<InpFam, MRes> for Multiple<Match>
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
        loop {
            let before = input.get_pos();
            if !runner.run_match(&self.matcher, error_handler, input)? {
                break;
            }
            if input.get_pos().into() == before.into() {
                break;
            }
        }
        Ok(true)
    }
}
