use crate::grammar::{
    context::MatchResult, error_handler::{ErrorHandler, MultiErrorHandler, ParserError}, matcher::{MatchRunner, Matcher, NoMemoizeBacktrackingRunner}
};

pub struct CommitMatcher<CommitOn, ThenMatch> {
    commit_on: CommitOn,
    then_matcher: ThenMatch,
}

impl<Commit, Match> CommitMatcher<Commit, Match> {
    pub fn new(commit_on: Commit, matcher: Match) -> Self {
        Self {
            commit_on,
            then_matcher: matcher,
        }
    }
}

impl<Token, MRes, CommitOn, ThenMatch> Matcher<Token, MRes> for CommitMatcher<CommitOn, ThenMatch>
where
    CommitOn: Matcher<Token, MRes>,
    ThenMatch: Matcher<Token, MRes>,
    MRes: MatchResult,
{
    const CAN_MATCH_DIRECTLY: bool =
        CommitOn::CAN_MATCH_DIRECTLY && ThenMatch::CAN_MATCH_DIRECTLY_ASSUMING_NO_FAIL;
    const HAS_PROPERTY: bool = CommitOn::HAS_PROPERTY || ThenMatch::HAS_PROPERTY;
    const CAN_FAIL: bool = CommitOn::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        if runner.run_match(&self.commit_on, error_handler, pos)? {
            if runner.run_match(&self.then_matcher, error_handler, pos)? {
                return Ok(true);
            }

            let mut error_handler = MultiErrorHandler::new(*pos);
            // starting new runner to find the error. we can't use the same runner again because then the properties of the match result will be written twice.
            let mut runner: NoMemoizeBacktrackingRunner<'_, '_, Token, MRes> = NoMemoizeBacktrackingRunner::new(runner.get_parser_context());
            runner.run_match(&self.then_matcher, &mut error_handler, pos)?;
            let err = error_handler.to_parser_error();
            return Err(err);
        }
        Ok(false)
    }
}

pub fn commit_on<CommitOn, ThenMatch>(
    commit_on: CommitOn,
    then_matcher: ThenMatch,
) -> CommitMatcher<CommitOn, ThenMatch> {
    CommitMatcher::new(commit_on, then_matcher)
}
