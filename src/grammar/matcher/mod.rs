pub mod any_token;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_of;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
pub mod positive_lookahead;
pub mod sequence;
pub mod string;
use std::ops::Deref;

use crate::grammar::{
    capture::BoundResult,
    context::{MatchResult, MatcherContext, ParserContext},
    error_handler::ErrorHandler,
};

pub trait Matcher<Token, MatchResult> {
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MatchResult, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String>;
}
pub trait ToMatcher {
    type MatcherType;
    fn to_matcher(&self) -> Self::MatcherType;
}

impl<Outer, Inner, Token, MRes> Matcher<Token, MRes> for Outer
where
    Outer: Deref<Target = Inner>,
    Inner: Matcher<Token, MRes>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}

pub trait CanMatchWithRunner<Runner> {
    fn match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String>;
}
pub trait MatchRunner<'a, 'ctx> {
    type Token: 'ctx;
    type MRes: MatchResult;
    type EHandler: ErrorHandler + 'ctx;

    fn run_match<Matcher>(&mut self, matcher: &Matcher, pos: &mut usize) -> Result<bool, String>
    where
        Matcher: CanMatchWithRunner<Self>,
        Self: Sized;

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res);

    fn get_match_result(self) -> Self::MRes;

    fn get_parser_context<'b>(
        &'b mut self,
    ) -> &'b mut ParserContext<'ctx, Self::Token, Self::EHandler>;
}

impl<'a, 'ctx, Token, MRes, EHandler: ErrorHandler>
    NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler>
{
    pub fn new(parser_context: &'a mut ParserContext<'ctx, Token, EHandler>) -> Self {
        Self {
            parser_context,
            stack: Vec::new(),
        }
    }
}

pub struct NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler: ErrorHandler> {
    parser_context: &'a mut ParserContext<'ctx, Token, EHandler>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'a>>,
}
impl<'a, 'ctx, Token, MRes, EHandler> MatchRunner<'a, 'ctx>
    for NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler>
where
    EHandler: ErrorHandler,
    MRes: MatchResult,
{
    type Token = Token;
    type MRes = MRes;
    type EHandler = EHandler;

    fn run_match<Matcher>(&mut self, matcher: &Matcher, pos: &mut usize) -> Result<bool, String>
    where
        Matcher: CanMatchWithRunner<Self>,
        Self: Sized,
    {
        let old_pos = *pos;
        let old_stack_len = self.stack.len();
        let result = matcher.match_with_runner(self, pos)?;

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

    fn get_parser_context<'b>(
        &'b mut self,
    ) -> &'b mut ParserContext<'ctx, Self::Token, Self::EHandler> {
        self.parser_context
    }
}

pub trait CanImplMatchWithRunner<Runner> {
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String>;
}

pub trait DoImplMatchWithNoMoemoizeBacktrackingRunner {}

impl<'a, 'ctx, T, Token, MRes, EHandler>
    CanMatchWithRunner<NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler>> for T
where
    T: DoImplMatchWithNoMoemoizeBacktrackingRunner
        + CanImplMatchWithRunner<NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler>>,
    EHandler: ErrorHandler,
    MRes: MatchResult,
{
    fn match_with_runner(
        &self,
        runner: &mut NoMoemoizeBacktrackingRunner<'a, 'ctx, Token, MRes, EHandler>,
        pos: &mut usize,
    ) -> Result<bool, String> {
        self.impl_match_with_runner(runner, pos)
    }
}
