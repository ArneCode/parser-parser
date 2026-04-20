use std::marker::PhantomData;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::Matcher,
    parser::capture::{BoundResult, MatchResult},
};

/// Runs matchers for one parse invocation over input lifetime `'src`.
///
/// `'a` is the lifetime of the runner borrow passed into nested `match_with_runner` calls.
/// Deferred captures are stored as [`BoundResult`] trait objects at lifetime **`'src`**
/// (the parse / input-stream invocation), so values may borrow the input for `'src`.
pub(crate) trait MatchRunner<'a, 'src, Inp>
where
    Inp: Input<'src>,
{
    type MRes: MatchResult;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<'src, Inp, Self::MRes>,
        'src: 'a,
        Self: Sized;

    fn register_result<Res: BoundResult<Self::MRes> + 'a>(&mut self, result: Res);

    fn get_match_result(self) -> Self::MRes;

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext;
}

pub(crate) struct NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes> {
    parser_context: &'a mut ParserContext,
    _phantom_inp: PhantomData<Inp>,
    _phantom_src: PhantomData<&'src ()>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'a>>,
}

impl<'a, 'src, Inp: Input<'src>, MRes> NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes> {
    pub(crate) fn new(
        parser_context: &'a mut ParserContext,
        _marker: PhantomData<&'src ()>,
    ) -> Self {
        Self {
            parser_context,
            _phantom_inp: PhantomData,
            _phantom_src: PhantomData,
            stack: Vec::new(),
        }
    }
}

impl<'a, 'src, Inp: Input<'src>, MRes> MatchRunner<'a, 'src, Inp>
    for NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes>
where
    MRes: MatchResult,
{
    type MRes = MRes;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<'src, Inp, Self::MRes>,
        'src: 'a,
        Self: Sized,
    {
        let old_pos = input.get_pos();
        let old_stack_len = self.stack.len();
        let idx = error_handler.register_start(old_pos.clone().into());
        let result = matcher.match_with_runner(self, error_handler, input)?;
        error_handler.register_watermark(input.get_pos().into());
        if !result {
            input.set_pos(old_pos);
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

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext {
        self.parser_context
    }
}

pub(crate) struct DirectMatchRunner<'a, 'src, Inp, MRes> {
    parser_context: &'a mut ParserContext,
    _phantom_inp: PhantomData<Inp>,
    _phantom_src: PhantomData<&'src ()>,
    result: MRes,
}

impl<'a, 'src, Inp: Input<'src>, MRes> DirectMatchRunner<'a, 'src, Inp, MRes> {
    pub(crate) fn new(
        parser_context: &'a mut ParserContext,
        result: MRes,
        _marker: PhantomData<&'src ()>,
    ) -> Self {
        Self {
            parser_context,
            _phantom_inp: PhantomData,
            _phantom_src: PhantomData,
            result,
        }
    }
}

impl<'a, 'src, Inp: Input<'src>, MRes> MatchRunner<'a, 'src, Inp> for DirectMatchRunner<'a, 'src, Inp, MRes>
where
    MRes: MatchResult,
{
    type MRes = MRes;

    fn run_match<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<'src, Inp, Self::MRes>,
        'src: 'a,
        Self: Sized,
    {
        let old_pos = input.get_pos();
        let idx = error_handler.register_start(old_pos.clone().into());
        if matcher.match_with_runner(self, error_handler, input)? {
            error_handler.register_success(idx);
            Ok(true)
        } else {
            input.set_pos(old_pos);
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

    fn get_parser_context<'b>(&'b mut self) -> &'b mut ParserContext {
        self.parser_context
    }
}
