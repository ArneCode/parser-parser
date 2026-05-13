//! Committing sequence: after `commit_on` succeeds, failure in `then_matcher` becomes a hard error.

use std::mem::swap;

use crate::{
    error::{
        MatcherRunError,
        error_handler::{ErrorHandler, MultiErrorHandler},
    },
    input::{Input, InputStream},
    matcher::{
        MatchRunner, Matcher, MatcherCombinator,
    },
    memo_store::MemoStore,
    parser::capture::MatchResult,
};

/// Runs `commit_on` then `then_matcher`; if the second fails, furthest-fail is surfaced as [`Err`].
#[derive(Clone, Debug)]
pub struct CommitMatcher<CommitOn, ThenMatch> {
    commit_on: CommitOn,
    then_matcher: ThenMatch,
}

impl<CommitOn, ThenMatch> MatcherCombinator for CommitMatcher<CommitOn, ThenMatch> {}

impl<Commit, Match> CommitMatcher<Commit, Match> {
    /// See [`commit_on`].
    pub fn new(commit_on: Commit, matcher: Match) -> Self {
        Self {
            commit_on,
            then_matcher: matcher,
        }
    }
}

impl<'src, Inp: Input<'src>, MRes, CommitOn, ThenMatch>
    super::internal::MatcherImpl<'src, Inp, MRes> for CommitMatcher<CommitOn, ThenMatch>
where
    CommitOn: Matcher<'src, Inp, MRes>,
    ThenMatch: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
    MRes: MatchResult,
{
    const CAN_MATCH_DIRECTLY: bool =
        CommitOn::CAN_MATCH_DIRECTLY && ThenMatch::CAN_MATCH_DIRECTLY_ASSUMING_NO_FAIL;
    const HAS_PROPERTY: bool = CommitOn::HAS_PROPERTY || ThenMatch::HAS_PROPERTY;
    const CAN_FAIL: bool = CommitOn::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if !runner.run_match(&self.commit_on, error_handler, input)? {
            return Ok(false);
        }
        if !runner.is_in_error_recovery_mode() {
            if runner.run_match(&self.then_matcher, error_handler, input)? {
                return Ok(true);
            }
            return Err(MatcherRunError::RetryRerunNeeded);
        }

        let mut inner_error_handler = MultiErrorHandler::new(input.get_pos().into());
        // use empty cache so that every Symbol is explored fully, otherwise we might miss some errors due to memoization.
        let mut cache = MemoStore::default();
        swap(&mut runner.get_parser_context().memo_store, &mut cache);
        let result = runner.run_match(&self.then_matcher, &mut inner_error_handler, input)?;
        swap(&mut runner.get_parser_context().memo_store, &mut cache);

        if result {
            Ok(true)
        } else {
            Err(MatcherRunError::FurthestFail(
                inner_error_handler.to_parser_error(),
            ))
        }
    }
}

/// Convenience constructor for [`CommitMatcher`].
pub fn commit_on<CommitOn, ThenMatch>(
    commit_on: CommitOn,
    then_matcher: ThenMatch,
) -> CommitMatcher<CommitOn, ThenMatch> {
    CommitMatcher::new(commit_on, then_matcher)
}
