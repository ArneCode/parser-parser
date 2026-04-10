// pub mod any_token;
pub mod multiple;
// pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
// pub mod positive_lookahead;
pub mod sequence;
pub mod string;
use std::ops::Deref;

use crate::grammar::{
    capture::BoundResult,
    context::{MatchResult, ParserContext},
    error_handler::{self, ErrorHandler, ParserError},
};

pub trait ToMatcher {
    type MatcherType;
    fn to_matcher(&self) -> Self::MatcherType;
}

pub trait CanMatchWithRunner<Runner> {
    fn match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>;
}
pub trait MatchRunner<'a, 'ctx> {
    type Token: 'ctx;
    type MRes: MatchResult;

    fn run_match<Matcher, EHandler: ErrorHandler>(
        &mut self,
        matcher: &Matcher,
        error_handler: &mut EHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>
    where
        Matcher: CanMatchWithRunner<Self>,
        Self: Sized;

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res);

    fn get_match_result(self) -> Self::MRes;

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext<'ctx, Self::Token>;
}

impl<'a, 'ctx, Token, MRes> NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    pub fn new(parser_context: &'a mut ParserContext<'ctx, Token>) -> Self {
        Self {
            parser_context,
            stack: Vec::new(),
        }
    }
}

pub struct NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    parser_context: &'a mut ParserContext<'ctx, Token>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'a>>,
}
impl<'a, 'ctx, Token, MRes> MatchRunner<'a, 'ctx>
    for NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes>
where
    MRes: MatchResult,
{
    type Token = Token;
    type MRes = MRes;

    fn run_match<Matcher, EHandler: ErrorHandler>(
        &mut self,
        matcher: &Matcher,
        error_handler: &mut EHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>
    where
        Matcher: CanMatchWithRunner<Self>,
        Self: Sized,
    {
        let old_pos = *pos;
        let old_stack_len = self.stack.len();
        let result = matcher.match_with_runner(self, error_handler, pos)?;
        error_handler.register_watermark(*pos);
        if !result {
            *pos = old_pos;
            self.stack.truncate(old_stack_len);
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

pub trait CanImplMatchWithRunner<Runner> {
    fn impl_match_with_runner(
        &self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError>;
}

pub trait DoImplMatchWithNoMoemoizeBacktrackingRunner {}

impl<'a, 'ctx, T, Token, MRes>
    CanMatchWithRunner<NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes>> for T
where
    T: DoImplMatchWithNoMoemoizeBacktrackingRunner
        + CanImplMatchWithRunner<NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes>>,
    MRes: MatchResult,
{
    fn match_with_runner(
        &self,
        runner: &mut NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
        self.impl_match_with_runner(runner, error_handler, pos)
    }
}
