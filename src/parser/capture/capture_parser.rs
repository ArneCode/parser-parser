use std::fmt::Display;
use std::marker::PhantomData;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{DirectMatchRunner, Matcher, NoMemoizeBacktrackingRunner, runner::MatchRunner},
    parser::{ParserCombinator, internal::ParserImpl},
};
#[cfg(feature = "parser-trace")]
use crate::trace::{RuleSourceMetadata, TraceEventKind};

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
    #[cfg(feature = "parser-trace")]
    pub(super) source: RuleSourceMetadata,
}

impl<MRes, Match, F> Clone for Capture<MRes, Match, F>
where
    Match: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            matcher: self.matcher.clone(),
            constructor: self.constructor.clone(),
            _phantom: PhantomData,
            #[cfg(feature = "parser-trace")]
            source: self.source,
        }
    }
}

impl<MRes, Match, F> std::fmt::Debug for Capture<MRes, Match, F>
where
    Match: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture")
            .field("matcher", &self.matcher)
            .finish()
    }
}

impl<MResSingle, MResMultiple, MResOptional, Match, F> ParserCombinator
    for Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
where
    Match: crate::matcher::MatcherCombinator,
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
    #[cfg_attr(feature = "parser-trace", track_caller)]
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
        #[cfg(feature = "parser-trace")]
        let caller = std::panic::Location::caller();
        Self {
            matcher: grammar_factory(properties_single, properties_multiple, properties_optional),
            constructor,
            _phantom: PhantomData,
            #[cfg(feature = "parser-trace")]
            source: RuleSourceMetadata::new(caller.file(), caller.line(), caller.column())
                .with_rule_name("capture"),
        }
    }

    #[cfg(feature = "parser-trace")]
    fn source_metadata(&self) -> RuleSourceMetadata {
        self.source
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
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out + Clone,
{
    type Output = Out;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start_pos: usize = input.get_pos().into();
        #[cfg(feature = "parser-trace")]
        if context.trace_enabled() {
            context.trace_enter(
                TraceEventKind::CaptureEnter,
                start_pos,
                Some("capture".to_string()),
                Some(self.source_metadata()),
            );
        }
        // let old_match_start = context.match_start;
        // context.match_start = *pos;
        let result = if Match::CAN_MATCH_DIRECTLY && !error_handler.is_real() {
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
        };
        #[cfg(feature = "parser-trace")]
        if context.trace_enabled() {
            context.trace_event(
                TraceEventKind::CaptureExit,
                start_pos,
                input.get_pos().into(),
                Some("capture".to_string()),
                Some(self.source_metadata()),
            );
            context.trace_leave();
        }
        result
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.matcher.maybe_label()
    }
}
