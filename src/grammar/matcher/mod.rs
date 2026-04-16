pub mod any_token;
pub mod commit_matcher;
pub mod error_contextualizer;
pub mod insert_on_error;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
pub mod positive_lookahead;
pub mod sequence;
pub mod string;

use std::{fmt::Display, ops::Deref, rc::Rc};

use crate::grammar::{
    capture::BoundResult,
    context::{MatchResult, ParserContext},
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{error_contextualizer::ErrorContextualizer, insert_on_error::InsertOnErrorMatcher},
    parser::Parser,
};

pub trait ToMatcher {
    type MatcherType;
    fn to_matcher(&self) -> Self::MatcherType;
}

pub trait Matcher<Token, MRes> {
    /// whether this matcher will always either succeed or fail without writing to the matchresult
    const CAN_MATCH_DIRECTLY: bool;
    const CAN_MATCH_DIRECTLY_ASSUMING_NO_FAIL: bool = Self::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool;
    const CAN_FAIL: bool;
    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx;
    fn add_error_info<Pars, F>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars, F, MRes>
    where
        Self: Sized,
        Pars: Parser<Token, Output = F>,
        F: Fn(&mut FurthestFailError),
    {
        ErrorContextualizer::new(self, error_parser)
    }
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        None
    }
    fn try_insert_if_missing<M: Display>(self, message: M) -> InsertOnErrorMatcher<Self>
    where
        Self: Sized,
    {
        InsertOnErrorMatcher {
            inner: self,
            message: message.to_string(),
        }
    }
}

impl<Token, MRes, Inner> Matcher<Token, MRes> for &Inner
where
    Inner: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        (**self).match_with_runner(runner, error_handler, pos)
    }
}

impl<Token, MRes, Inner> Matcher<Token, MRes> for Rc<Inner>
where
    Inner: Matcher<Token, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        self.deref().match_with_runner(runner, error_handler, pos)
    }
}

pub trait MatchRunner<'a, 'ctx> {
    type Token: 'ctx;
    type MRes: MatchResult;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<Self::Token, Self::MRes>,
        Self: Sized;

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res);

    fn get_match_result(self) -> Self::MRes;

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext<'ctx, Self::Token>;
}

impl<'a, 'ctx, Token, MRes> NoMemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    pub const fn new(parser_context: &'a mut ParserContext<'ctx, Token>) -> Self {
        Self {
            parser_context,
            stack: Vec::new(),
        }
    }
}

pub struct NoMemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    parser_context: &'a mut ParserContext<'ctx, Token>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'a>>,
}
impl<'a, 'ctx, Token, MRes> MatchRunner<'a, 'ctx>
    for NoMemoizeBacktrackingRunner<'a, 'ctx, Token, MRes>
where
    MRes: MatchResult,
{
    type Token = Token;
    type MRes = MRes;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<Self::Token, Self::MRes>,
        Self: Sized,
    {
        let old_pos = *pos;
        let old_stack_len = self.stack.len();
        let idx = error_handler.register_start(*pos);
        let result = matcher.match_with_runner(self, error_handler, pos)?;
        error_handler.register_watermark(*pos);
        if !result {
            *pos = old_pos;
            self.stack.truncate(old_stack_len);
            error_handler.register_failure(
                matcher.maybe_label(),
                idx,
                self.parser_context.match_start,
            );
        } else {
            error_handler.register_success(idx);
        }
        Ok(result)
    }

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res) {
        self.stack.push(Box::new(result));
    }

    fn get_match_result(self) -> Self::MRes {
        let mut mres = Self::MRes::new_empty();
        for res in self.stack.into_iter() {
            res.put_boxed_in_result(&mut mres);
        }
        mres
    }

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext<'ctx, Self::Token> {
        self.parser_context
    }
}

pub struct DirectMatchRunner<'a, 'ctx, Token, MRes> {
    parser_context: &'a mut ParserContext<'ctx, Token>,
    result: MRes,
}

impl<'a, 'ctx, Token, MRes> MatchRunner<'a, 'ctx> for DirectMatchRunner<'a, 'ctx, Token, MRes>
where
    MRes: MatchResult,
{
    type Token = Token;
    type MRes = MRes;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<Self::Token, Self::MRes>,
        Self: Sized,
    {
        let old_pos = *pos;
        let idx = error_handler.register_start(*pos);
        if matcher.match_with_runner(self, error_handler, pos)? {
            error_handler.register_success(idx);
            Ok(true)
        } else {
            *pos = old_pos;
            error_handler.register_failure(
                matcher.maybe_label(),
                idx,
                self.parser_context.match_start,
            );
            Ok(false)
        }
    }

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res) {
        result.put_in_result(&mut self.result);
    }

    fn get_match_result(self) -> Self::MRes {
        self.result
    }

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext<'ctx, Self::Token> {
        self.parser_context
    }
}

impl<'a, 'ctx, Token, MRes> DirectMatchRunner<'a, 'ctx, Token, MRes> {
    pub fn new(parser_context: &'a mut ParserContext<'ctx, Token>, result: MRes) -> Self {
        Self {
            parser_context,
            result,
        }
    }
}

impl<Token, MRes> Matcher<Token, MRes> for () {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = false;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        _pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        Ok(true)
    }
}
