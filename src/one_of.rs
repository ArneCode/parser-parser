//! Ordered choice: try each alternative until one succeeds.
//!
//! [`OneOf`] implements both [`crate::parser::Parser`] and [`crate::matcher::Matcher`]
//! for tuples of alternatives. It lives at the crate root (not under [`crate::parser`]
//! or [`crate::matcher`]) so it can depend on both without creating a dependency cycle.

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, Matcher, MatcherCombinator},
    parser::{Parser, ParserCombinator},
};
#[cfg(feature = "parser-trace")]
use crate::trace::{RuleSourceMetadata, TraceEventKind};

/// Wraps a tuple of parsers or matchers; used with [`one_of`].
#[derive(Clone, Debug)]
pub struct OneOf<Tuple> {
    options: Tuple,
    #[cfg(feature = "parser-trace")]
    source: RuleSourceMetadata,
}

impl<Tuple> OneOf<Tuple> {
    /// Builds an alternative group from a tuple `(first, second, …)`.
    #[cfg(feature = "parser-trace")]
    #[track_caller]
    pub fn new(options: Tuple) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            options,
            source: RuleSourceMetadata::new(caller.file(), caller.line(), caller.column()),
        }
    }

    /// Builds an alternative group from a tuple `(first, second, …)`.
    #[cfg(not(feature = "parser-trace"))]
    pub fn new(options: Tuple) -> Self {
        Self { options }
    }

    #[cfg(feature = "parser-trace")]
    fn source_metadata(&self) -> RuleSourceMetadata {
        self.source
    }
}

/// Convenience alias for [`OneOf::new`].
#[cfg(feature = "parser-trace")]
#[track_caller]
pub fn one_of<Options>(options: Options) -> OneOf<Options> {
    OneOf::new(options)
}

/// Convenience alias for [`OneOf::new`].
#[cfg(not(feature = "parser-trace"))]
pub fn one_of<Options>(options: Options) -> OneOf<Options> {
    OneOf::new(options)
}

macro_rules! impl_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<$head, $($tail),*> MatcherCombinator for OneOf<($head, $($tail,)*)> where
            $head: MatcherCombinator,
            $($tail: MatcherCombinator,)*
        {}
        impl<$head, $($tail),*> ParserCombinator for OneOf<($head, $($tail,)*)> where
            $head: ParserCombinator,
            $($tail: ParserCombinator,)*
        {}
        impl<'src, Inp: Input<'src>, MRes, $head, $($tail),*> crate::matcher::internal::MatcherImpl<'src, Inp, MRes> for OneOf<($head, $($tail,)*)>
        where
            $head: Matcher<'src, Inp, MRes>,
            $($tail: Matcher<'src, Inp, MRes>,)*
            Inp: Input<'src>,
        {
            const CAN_MATCH_DIRECTLY: bool = $head::CAN_MATCH_DIRECTLY  $(&& $tail::CAN_MATCH_DIRECTLY)*;
            const HAS_PROPERTY: bool = $head::HAS_PROPERTY  $(|| $tail::HAS_PROPERTY)*;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;

            fn match_with_runner<'a, Runner>(&'a self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, input: &mut InputStream<'src, Inp>) -> Result<bool, FurthestFailError>
            where
                Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
                'src: 'a,
            {
                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;
                #[cfg(feature = "parser-trace")]
                {
                    runner.get_parser_context().trace_event(
                        TraceEventKind::ChoiceStart,
                        input.get_pos().into(),
                        input.get_pos().into(),
                        None,
                        Some(self.source_metadata()),
                    );
                }

                #[cfg(feature = "parser-trace")]
                let mut arm_idx = 0usize;
                #[cfg(feature = "parser-trace")]
                runner.get_parser_context().trace_event(
                    TraceEventKind::ChoiceArmStart,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    Some(format!("arm {arm_idx}")),
                    Some(self.source_metadata()),
                );
                if runner.run_match($head, error_handler, input)? {
                    #[cfg(feature = "parser-trace")]
                    runner.get_parser_context().trace_event(
                        TraceEventKind::ChoiceArmSuccess,
                        input.get_pos().into(),
                        input.get_pos().into(),
                        Some(format!("arm {arm_idx}")),
                        Some(self.source_metadata()),
                    );
                    return Ok(true);
                }
                #[cfg(feature = "parser-trace")]
                runner.get_parser_context().trace_event(
                    TraceEventKind::ChoiceArmFail,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    Some(format!("arm {arm_idx}")),
                    Some(self.source_metadata()),
                );

                $(
                    #[cfg(feature = "parser-trace")]
                    {
                        arm_idx += 1;
                        runner.get_parser_context().trace_event(
                            TraceEventKind::ChoiceArmStart,
                            input.get_pos().into(),
                            input.get_pos().into(),
                            Some(format!("arm {arm_idx}")),
                            Some(self.source_metadata()),
                        );
                    }
                    if runner.run_match($tail, error_handler, input)? {
                        #[cfg(feature = "parser-trace")]
                        runner.get_parser_context().trace_event(
                            TraceEventKind::ChoiceArmSuccess,
                            input.get_pos().into(),
                            input.get_pos().into(),
                            Some(format!("arm {arm_idx}")),
                            Some(self.source_metadata()),
                        );
                        return Ok(true);
                    }
                    #[cfg(feature = "parser-trace")]
                    runner.get_parser_context().trace_event(
                        TraceEventKind::ChoiceArmFail,
                        input.get_pos().into(),
                        input.get_pos().into(),
                        Some(format!("arm {arm_idx}")),
                        Some(self.source_metadata()),
                    );
                )*

                #[cfg(feature = "parser-trace")]
                runner.get_parser_context().trace_event(
                    TraceEventKind::ChoiceAllFailed,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    None,
                    Some(self.source_metadata()),
                );
                Ok(false)
            }
        }
        impl<'src, Inp: Input<'src>, Output, $head, $($tail),*> crate::parser::internal::ParserImpl<'src, Inp> for OneOf<($head, $($tail,)*)>
        where
            $head: Parser<'src, Inp, Output = Output>,
            $($tail: Parser<'src, Inp, Output = Output>,)*
            Inp: Input<'src>,
        {
            type Output = Output;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;
            fn parse(&self, context: &mut ParserContext, error_handler: &mut impl ErrorHandler, input: &mut InputStream<'src, Inp>) -> Result<Option<Output>, FurthestFailError> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;
                #[cfg(feature = "parser-trace")]
                context.trace_event(
                    TraceEventKind::ChoiceStart,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    None,
                    Some(self.source_metadata()),
                );

                // if $head.check_no_advance(context, pos) {
                //     return $head.parse(context, pos);
                // }
                #[cfg(feature = "parser-trace")]
                let mut arm_idx = 0usize;
                #[cfg(feature = "parser-trace")]
                context.trace_event(
                    TraceEventKind::ChoiceArmStart,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    Some(format!("arm {arm_idx}")),
                    Some(self.source_metadata()),
                );
                if let Some(output) = $head.parse(context, error_handler, input)? {
                    #[cfg(feature = "parser-trace")]
                    context.trace_event(
                        TraceEventKind::ChoiceArmSuccess,
                        input.get_pos().into(),
                        input.get_pos().into(),
                        Some(format!("arm {arm_idx}")),
                        Some(self.source_metadata()),
                    );
                    return Ok(Some(output));
                }
                #[cfg(feature = "parser-trace")]
                context.trace_event(
                    TraceEventKind::ChoiceArmFail,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    Some(format!("arm {arm_idx}")),
                    Some(self.source_metadata()),
                );

                // $(
                //     if $tail.check_no_advance(context, pos) {
                //         return $tail.parse(context, pos);
                //     }
                // )*
                $(
                    #[cfg(feature = "parser-trace")]
                    {
                        arm_idx += 1;
                        context.trace_event(
                            TraceEventKind::ChoiceArmStart,
                            input.get_pos().into(),
                            input.get_pos().into(),
                            Some(format!("arm {arm_idx}")),
                            Some(self.source_metadata()),
                        );
                    }
                    if let Some(output) = $tail.parse(context, error_handler, input)? {
                        #[cfg(feature = "parser-trace")]
                        context.trace_event(
                            TraceEventKind::ChoiceArmSuccess,
                            input.get_pos().into(),
                            input.get_pos().into(),
                            Some(format!("arm {arm_idx}")),
                            Some(self.source_metadata()),
                        );
                        return Ok(Some(output));
                    }
                    #[cfg(feature = "parser-trace")]
                    context.trace_event(
                        TraceEventKind::ChoiceArmFail,
                        input.get_pos().into(),
                        input.get_pos().into(),
                        Some(format!("arm {arm_idx}")),
                        Some(self.source_metadata()),
                    );
                )*

                #[cfg(feature = "parser-trace")]
                context.trace_event(
                    TraceEventKind::ChoiceAllFailed,
                    input.get_pos().into(),
                    input.get_pos().into(),
                    None,
                    Some(self.source_metadata()),
                );
                Ok(None)
            }
        }
        impl_one_of_tuples!($($tail),*);
    };
}

impl_one_of_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
