//! Ordered choice: try each alternative until one succeeds.
//!
//! [`OneOf`] implements both [`crate::parser::Parser`] and [`crate::matcher::Matcher`]
//! for tuples of alternatives. It lives at the crate root (not under [`crate::parser`]
//! or [`crate::matcher`]) so it can depend on both without creating a dependency cycle.

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};

/// Wraps a tuple of parsers or matchers; used with [`one_of`].
pub struct OneOf<Tuple> {
    options: Tuple,
}

impl<Tuple> OneOf<Tuple> {
    /// Builds an alternative group from a tuple `(first, second, …)`.
    pub fn new(options: Tuple) -> Self {
        Self { options }
    }
}

/// Convenience alias for [`OneOf::new`].
pub fn one_of<Options>(options: Options) -> OneOf<Options> {
    OneOf::new(options)
}

macro_rules! impl_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<InpFam, MRes, $head, $($tail),*> crate::matcher::internal::MatcherImpl<InpFam, MRes> for OneOf<($head, $($tail,)*)>
        where
            InpFam: InputFamily + ?Sized,
            $head: Matcher<InpFam, MRes>,
            $($tail: Matcher<InpFam, MRes>,)*
        {
            const CAN_MATCH_DIRECTLY: bool = $head::CAN_MATCH_DIRECTLY  $(&& $tail::CAN_MATCH_DIRECTLY)*;
            const HAS_PROPERTY: bool = $head::HAS_PROPERTY  $(|| $tail::HAS_PROPERTY)*;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;

            fn match_with_runner<'a, 'src, Runner>(&'a self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, input: &mut InputStream<'src, InpFam::In<'src>>) -> Result<bool, FurthestFailError>
            where
                Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
                'src: 'a,
            {
                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if runner.run_match($head, error_handler, input)? {
                    return Ok(true);
                }

                $(
                    if runner.run_match($tail, error_handler, input)? {
                        return Ok(true);
                    }
                )*

                Ok(false)
            }
        }
        impl<InpFam, Output, $head, $($tail),*> crate::parser::internal::ParserImpl<InpFam> for OneOf<($head, $($tail,)*)>
        where
            InpFam: InputFamily + ?Sized,
            $head: for<'src> Parser<InpFam, Output<'src> = Output>,
            $($tail: for<'src> Parser<InpFam, Output<'src> = Output>,)*
        {
            type Output<'src> = Output;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;
            fn parse<'src>(&self, context: &mut ParserContext, error_handler: &mut impl ErrorHandler, input: &mut InputStream<'src, InpFam::In<'src>>) -> Result<Option<Output>, FurthestFailError> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                // if $head.check_no_advance(context, pos) {
                //     return $head.parse(context, pos);
                // }
                if let Some(output) = $head.parse(context, error_handler, input)? {
                    return Ok(Some(output));
                }

                // $(
                //     if $tail.check_no_advance(context, pos) {
                //         return $tail.parse(context, pos);
                //     }
                // )*
                $(
                    if let Some(output) = $tail.parse(context, error_handler, input)? {
                        return Ok(Some(output));
                    }
                )*

                Ok(None)
            }
        }
        impl_one_of_tuples!($($tail),*);
    };
}

impl_one_of_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
