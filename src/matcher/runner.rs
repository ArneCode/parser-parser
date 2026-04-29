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
    fn run_match_inner<Match, EHandler: ErrorHandler>(
        &mut self,
        matcher: &'a Match,
        error_handler: &mut EHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Match: Matcher<'src, Inp, Self::MRes>,
        'src: 'a,
        Self: Sized;
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
        let old_stack_len = self.get_parser_context().error_stack.len();
        let result = self.run_match_inner(matcher, error_handler, input);
        error_handler.register_watermark(input.get_pos().into());
        let result = if let Err(err) = result {
            error_handler.register_failure(matcher.maybe_label(), idx);
            // move back error stack to the previous state
            self.get_parser_context()
                .error_stack
                .truncate(old_stack_len);
            return Err(err);
        } else {
            result.unwrap()
        };
        if !result {
            input.set_pos(old_pos);
            error_handler.register_failure(matcher.maybe_label(), idx);
            // move back error stack to the previous state
            self.get_parser_context()
                .error_stack
                .truncate(old_stack_len);
        } else {
            error_handler.register_success(idx);
        }
        Ok(result)
    }

    fn register_result<Res: BoundResult<Self::MRes> + 'src>(&mut self, result: Res);

    fn get_match_result(self) -> Self::MRes;

    fn get_parser_context(&mut self) -> &mut ParserContext;

    fn apply_results(&mut self, results: Vec<Box<dyn BoundResult<Self::MRes> + 'src>>);
    fn maybe_get_as_direct_match_runner(
        &mut self,
    ) -> Option<&mut DirectMatchRunner<'a, 'src, Inp, Self::MRes>> {
        None
    }
}

pub(crate) struct NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes> {
    parser_context: &'a mut ParserContext,
    _phantom: PhantomData<(&'src (), Inp)>,
    stack: Vec<Box<dyn BoundResult<MRes> + 'src>>,
}

impl<'a, 'src, Inp: Input<'src>, MRes> NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes> {
    pub(crate) fn new(parser_context: &'a mut ParserContext) -> Self {
        Self {
            parser_context,
            _phantom: PhantomData,
            stack: Vec::new(),
        }
    }
}

impl<'a, 'src, Inp: Input<'src>, MRes> NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes> {
    pub(crate) fn get_data(
        self,
    ) -> (
        &'a mut ParserContext,
        Vec<Box<dyn BoundResult<MRes> + 'src>>,
    ) {
        (self.parser_context, self.stack)
    }
}

impl<'a, 'src, Inp: Input<'src>, MRes> MatchRunner<'a, 'src, Inp>
    for NoMemoizeBacktrackingRunner<'a, 'src, Inp, MRes>
where
    MRes: MatchResult,
{
    type MRes = MRes;

    fn run_match_inner<Match, EHandler: ErrorHandler>(
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
        let old_stack_len = self.stack.len();
        let result = matcher.match_with_runner(self, error_handler, input)?;
        if !result {
            self.stack.truncate(old_stack_len);
        }
        Ok(result)
    }

    fn register_result<Res: BoundResult<Self::MRes> + 'src>(&mut self, result: Res) {
        self.stack.push(Box::new(result));
    }

    fn get_match_result(self) -> Self::MRes {
        let mut mres = Self::MRes::new_empty();
        for res in self.stack.into_iter() {
            res.put_boxed_in_result(&mut mres);
        }
        mres
    }

    fn get_parser_context(&mut self) -> &mut ParserContext {
        self.parser_context
    }

    fn apply_results(&mut self, results: Vec<Box<dyn BoundResult<Self::MRes> + 'src>>) {
        self.stack.extend(results);
    }
}

pub(crate) struct DirectMatchRunner<'a, 'src, Inp, MRes> {
    parser_context: &'a mut ParserContext,
    _phantom: PhantomData<(&'src (), Inp)>,
    result: MRes,
}

impl<'a, 'src, Inp: Input<'src>, MRes> DirectMatchRunner<'a, 'src, Inp, MRes> {
    pub(crate) fn new(parser_context: &'a mut ParserContext) -> Self
    where
        MRes: MatchResult,
    {
        Self {
            parser_context,
            _phantom: PhantomData,
            result: MRes::new_empty(),
        }
    }

    pub(crate) fn get_match_result_mut(&mut self) -> &mut MRes {
        &mut self.result
    }
}

impl<'a, 'src, Inp: Input<'src>, MRes> MatchRunner<'a, 'src, Inp>
    for DirectMatchRunner<'a, 'src, Inp, MRes>
where
    MRes: MatchResult,
{
    type MRes = MRes;

    fn run_match_inner<Match, EHandler: ErrorHandler>(
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
        matcher.match_with_runner(self, error_handler, input)
    }

    fn register_result<Res: BoundResult<Self::MRes> + 'src>(&mut self, result: Res) {
        result.put_in_result(&mut self.result);
    }

    fn get_match_result(self) -> Self::MRes {
        self.result
    }

    fn get_parser_context(&mut self) -> &mut ParserContext {
        self.parser_context
    }

    fn apply_results(&mut self, results: Vec<Box<dyn BoundResult<Self::MRes> + 'src>>) {
        for result in results {
            result.put_boxed_in_result(&mut self.result);
        }
    }

    fn maybe_get_as_direct_match_runner(
        &mut self,
    ) -> Option<&mut DirectMatchRunner<'a, 'src, Inp, Self::MRes>> {
        Some(self)
    }
}
