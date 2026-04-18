//! Zero-or-more repetition matcher; stops when `matcher` fails or makes no progress.

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
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

impl<Token, MRes, Match> super::internal::MatcherImpl<Token, MRes> for Multiple<Match>
where
    Match: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Match::HAS_PROPERTY;
    const CAN_FAIL: bool = false;
    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
    {
        loop {
            let before = *pos;
            if !runner.run_match(&self.matcher, error_handler, pos)? {
                break;
            }
            if *pos == before {
                break;
            }
        }
        Ok(true)
    }
}
