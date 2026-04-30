use std::fmt::Display;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{DirectMatchRunner, Matcher, NoMemoizeBacktrackingRunner, runner::MatchRunner},
    parser::{ParserCombinator, internal::ParserImpl},
};
#[cfg(feature = "parser-trace")]
use crate::trace::{RuleSourceMetadata, TraceEventKind};

/// Parser produced by [`MatcherCombinator::to`](super::MatcherCombinator::to).
///
/// It runs a matcher and returns a fixed output value when the matcher succeeds.
/// The matcher is evaluated with an empty capture result, so this is intended for
/// ordinary matchers, not `bind!`/`bind_span!`/`bind_slice!` capture binders.
#[derive(Clone)]
pub struct ToParser<Match, Output> {
    matcher: Match,
    output: Output,
    #[cfg(feature = "parser-trace")]
    source: RuleSourceMetadata,
}

impl<Match, Output> ToParser<Match, Output> {
    #[cfg(feature = "parser-trace")]
    #[track_caller]
    pub fn new(matcher: Match, output: Output) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            matcher,
            output,
            source: RuleSourceMetadata::new(caller.file(), caller.line(), caller.column()),
        }
    }

    #[cfg(not(feature = "parser-trace"))]
    pub fn new(matcher: Match, output: Output) -> Self {
        Self { matcher, output }
    }

    #[cfg(feature = "parser-trace")]
    fn source_metadata(&self) -> RuleSourceMetadata {
        self.source
    }
}

impl<Match, Output> std::fmt::Debug for ToParser<Match, Output>
where
    Match: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToParser")
            .field("matcher", &self.matcher)
            .finish()
    }
}

impl<Match, Output> ParserCombinator for ToParser<Match, Output> {}

impl<'src, Inp, Match, Output> ParserImpl<'src, Inp> for ToParser<Match, Output>
where
    Inp: Input<'src>,
    Match: Matcher<'src, Inp, ((), (), ())>,
    Output: Clone,
{
    type Output = Output;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let start: usize = input.get_pos().into();
        #[cfg(feature = "parser-trace")]
        if context.trace_enabled() {
            context.trace_event(
                TraceEventKind::ParserEnter,
                start,
                start,
                Some("to_parser".to_string()),
                Some(self.source_metadata()),
            );
        }
        let result = if Match::CAN_MATCH_DIRECTLY && !error_handler.is_real() {
            let mut runner = DirectMatchRunner::new(context);
            if runner.run_match(&self.matcher, error_handler, input)? {
                Ok(Some(self.output.clone()))
            } else {
                Ok(None)
            }
        } else {
            let mut runner = NoMemoizeBacktrackingRunner::new(context);
            if runner.run_match(&self.matcher, error_handler, input)? {
                Ok(Some(self.output.clone()))
            } else {
                Ok(None)
            }
        };
        #[cfg(feature = "parser-trace")]
        if context.trace_enabled() {
            context.trace_event(
                TraceEventKind::ParserExit,
                start,
                input.get_pos().into(),
                Some("to_parser".to_string()),
                Some(self.source_metadata()),
            );
        }
        result
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.matcher.maybe_label()
    }
}
