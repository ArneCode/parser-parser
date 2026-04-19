use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::Matcher,
    parser::capture::{BoundResult, MatchResult},
};

pub(crate) trait MatchRunner<'a, 'ctx> {
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

pub(crate) struct NoMemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    parser_context: &'a mut ParserContext<'ctx, Token>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'a>>,
}

impl<'a, 'ctx, Token, MRes> NoMemoizeBacktrackingRunner<'a, 'ctx, Token, MRes> {
    pub(crate) const fn new(parser_context: &'a mut ParserContext<'ctx, Token>) -> Self {
        Self {
            parser_context,
            stack: Vec::new(),
        }
    }
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
            error_handler.register_failure(matcher.maybe_label(), idx);
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

pub(crate) struct DirectMatchRunner<'a, 'ctx, Token, MRes> {
    parser_context: &'a mut ParserContext<'ctx, Token>,
    result: MRes,
}

impl<'a, 'ctx, Token, MRes> DirectMatchRunner<'a, 'ctx, Token, MRes> {
    pub(crate) fn new(parser_context: &'a mut ParserContext<'ctx, Token>, result: MRes) -> Self {
        Self {
            parser_context,
            result,
        }
    }
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
            error_handler.register_failure(matcher.maybe_label(), idx);
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
