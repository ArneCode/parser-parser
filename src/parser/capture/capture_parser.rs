use std::marker::PhantomData;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{DirectMatchRunner, Matcher, NoMemoizeBacktrackingRunner, runner::MatchRunner},
    parser::{ParserCombinator, internal::ParserImpl},
};

use super::match_result::{MatchResultMultiple, MatchResultOptional, MatchResultSingle};

/// Parser that runs `matcher` and, on success, calls `constructor` with the filled capture buckets.
///
/// `MRes` is a triple `(single, multiple, optional)` of match-result pieces; the macro-generated
/// grammar usually matches this shape. See the [`super`] module for bind helpers and for using
/// `capture!` to build this type.
pub struct Capture<MRes, Match, F> {
    pub(super) matcher: Match,
    pub(super) constructor: F,
    pub(super) _phantom: PhantomData<MRes>,
}

impl<MResSingle, MResMultiple, MResOptional, Match, F> ParserCombinator
    for Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
{
}

impl<Out, MResSingle, MResMultiple, MResOptional, Match, F>
    Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out,
{
    /// Builds a capture parser: `grammar_factory` receives empty property slots and must return
    /// the matcher; `constructor` maps filled results to `Out`.
    pub fn new<
        'a,
        'ctx: 'a,
        GF: FnOnce(MResSingle::Properties, MResMultiple::Properties, MResOptional::Properties) -> Match,
    >(
        grammar_factory: GF,
        constructor: F,
    ) -> Self {
        let properties_single = MResSingle::new_properties();
        let properties_multiple = MResMultiple::new_properties();
        let properties_optional = MResOptional::new_properties();
        Self {
            matcher: grammar_factory(properties_single, properties_multiple, properties_optional),
            constructor,
            _phantom: PhantomData,
        }
    }
}

impl<'src, Inp: Input<'src>, Out, MResSingle, MResMultiple, MResOptional, Match, F>
    ParserImpl<'src, Inp> for Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    Match: Matcher<'src, Inp, (MResSingle, MResMultiple, MResOptional)>,
    Inp: Input<'src>,
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out,
{
    type Output = Out;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        // let old_match_start = context.match_start;
        // context.match_start = *pos;
        if Match::CAN_MATCH_DIRECTLY && !error_handler.is_real() {
            let mut runner = DirectMatchRunner::new(context);
            if runner.run_match(&self.matcher, error_handler, input)? {
                let (res_single, res_multiple, res_optional) = runner.get_match_result();
                let result = (self.constructor)(res_single.to_output(), res_multiple, res_optional);
                // context.match_start = old_match_start;
                Ok(Some(result))
            } else {
                drop(runner);
                // context.match_start = old_match_start;
                Ok(None)
            }
        } else {
            let mut runner = NoMemoizeBacktrackingRunner::new(context);
            if runner.run_match(&self.matcher, error_handler, input)? {
                let (res_single, res_multiple, res_optional) = runner.get_match_result();
                let result = (self.constructor)(res_single.to_output(), res_multiple, res_optional);
                // context.match_start = old_match_start;
                Ok(Some(result))
            } else {
                drop(runner);
                // context.match_start = old_match_start;
                Ok(None)
            }
        }
    }
}
