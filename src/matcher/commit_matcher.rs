//! Committing sequence: after `commit_on` succeeds, failure in `then_matcher` becomes a hard error.

use std::marker::PhantomData;

use crate::{
    error::{
        FurthestFailError,
        error_handler::{ErrorHandler, MultiErrorHandler},
    },
    input::{InputFamily, InputStream},
    matcher::{MatchRunner, Matcher, NoMemoizeBacktrackingRunner},
    parser::capture::MatchResult,
};

/// Runs `commit_on` then `then_matcher`; if the second fails, furthest-fail is surfaced as [`Err`].
pub struct CommitMatcher<CommitOn, ThenMatch> {
    commit_on: CommitOn,
    then_matcher: ThenMatch,
}

impl<Commit, Match> CommitMatcher<Commit, Match> {
    /// See [`commit_on`].
    pub fn new(commit_on: Commit, matcher: Match) -> Self {
        Self {
            commit_on,
            then_matcher: matcher,
        }
    }
}

impl<InpFam, MRes, CommitOn, ThenMatch> super::internal::MatcherImpl<InpFam, MRes>
    for CommitMatcher<CommitOn, ThenMatch>
where
    InpFam: InputFamily + ?Sized,
    CommitOn: Matcher<InpFam, MRes>,
    ThenMatch: Matcher<InpFam, MRes>,
    MRes: MatchResult,
{
    const CAN_MATCH_DIRECTLY: bool =
        CommitOn::CAN_MATCH_DIRECTLY && ThenMatch::CAN_MATCH_DIRECTLY_ASSUMING_NO_FAIL;
    const HAS_PROPERTY: bool = CommitOn::HAS_PROPERTY || ThenMatch::HAS_PROPERTY;
    const CAN_FAIL: bool = CommitOn::CAN_FAIL;

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
        if runner.run_match(&self.commit_on, error_handler, input)? {
            if runner.run_match(&self.then_matcher, error_handler, input)? {
                return Ok(true);
            }

            let mut error_handler = MultiErrorHandler::new(input.get_pos().into());
            // starting new runner to find the error. we can't use the same runner again because then the properties of the match result will be written twice.
            let mut inner_runner: NoMemoizeBacktrackingRunner<'_, 'src, InpFam, MRes> =
                NoMemoizeBacktrackingRunner::new(runner.get_parser_context(), PhantomData::<&'src ()>);
            inner_runner.run_match(&self.then_matcher, &mut error_handler, input)?;
            let err = error_handler.to_parser_error();
            return Err(err);
        }
        Ok(false)
    }
}

/// Convenience constructor for [`CommitMatcher`].
pub fn commit_on<CommitOn, ThenMatch>(
    commit_on: CommitOn,
    then_matcher: ThenMatch,
) -> CommitMatcher<CommitOn, ThenMatch> {
    CommitMatcher::new(commit_on, then_matcher)
}
