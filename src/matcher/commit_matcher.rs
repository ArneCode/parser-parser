//! Committing sequence: after `commit_on` succeeds, failure in `then_matcher` becomes a hard error.

use std::{collections::HashMap, mem::swap};

use crate::{
    error::{
        FurthestFailError,
        error_handler::{ErrorHandler, MultiErrorHandler},
    },
    input::{Input, InputStream},
    matcher::{DirectMatchRunner, MatchRunner, Matcher, MatcherCombinator, NoMemoizeBacktrackingRunner},
    parser::capture::MatchResult,
};

/// Runs `commit_on` then `then_matcher`; if the second fails, furthest-fail is surfaced as [`Err`].
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
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if runner.run_match(&self.commit_on, error_handler, input)? {
            if runner.run_match(&self.then_matcher, error_handler, input)? {
                return Ok(true);
            }
            if let Some(direct_runner) = runner.maybe_get_as_direct_match_runner() {
                let mut undo_runner = DirectMatchRunner::new(direct_runner.get_parser_context());
                undo_runner.run_match(&self.then_matcher, error_handler, input)?;
                let results = undo_runner.get_match_result();
                let old_result = direct_runner.get_match_result_mut();
                results.subtract_from_result(old_result);
            }

            let mut error_handler = MultiErrorHandler::new(input.get_pos().into());
            // starting new runner to find the error. we can't use the same runner again because then the properties of the match result will be written twice.
            let context = runner.get_parser_context();
            // use empty cache so that every Symbol is explored fully, otherwise we might miss some errors due to memoization.
            let mut cache = HashMap::new();
            swap(&mut context.memo_table, &mut cache);
            let mut inner_runner: NoMemoizeBacktrackingRunner<'_, 'src, Inp, MRes> =
                NoMemoizeBacktrackingRunner::new(context);
            let result = inner_runner.run_match(&self.then_matcher, &mut error_handler, input)?;
            // swap back cache so that the original runner can continue to use it.
            swap(&mut inner_runner.get_parser_context().memo_table, &mut cache);
            if result {
                let results = {
                    let (context, results) = inner_runner.get_data();
                    // error recovery succeeded
                    // writing stack errors that have been fixed during recovery.
                    error_handler.write_stack_errors(context);
                    results
                };
                runner.apply_results(results);
                return Ok(true);
            } else {
                let (_, results) = inner_runner.get_data();
                runner.apply_results(results);
                let err = error_handler.to_parser_error();
                return Err(err);
            }
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
