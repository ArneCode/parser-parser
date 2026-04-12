use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
};
pub struct OneOfMatcher<Tuple> {
    options: Tuple,
}

impl<Tuple> OneOfMatcher<Tuple> {
    pub fn new(options: Tuple) -> Self {
        Self { options }
    }
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<'a, 'ctx, Runner, $head, $($tail),*> Matcher<Runner> for OneOfMatcher<($head, $($tail,)*)>
        where
            Runner: MatchRunner<'a, 'ctx>,
            $head: Matcher<Runner>,
            $($tail: Matcher<Runner>,)*
        {
            const CAN_MATCH_DIRECTLY: bool = $head::CAN_MATCH_DIRECTLY  $(&& $tail::CAN_MATCH_DIRECTLY)*;
            const HAS_PROPERTY: bool = $head::HAS_PROPERTY  $(|| $tail::HAS_PROPERTY)*;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;

            fn match_with_runner(&self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<bool, ParserError> {
                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if runner.run_match($head, error_handler, pos)? {
                    return Ok(true);
                }

                $(
                    if runner.run_match($tail, error_handler, pos)? {
                        return Ok(true);
                    }
                )*

                Ok(false)
            }
        }

        impl_matcher_for_one_of_tuples!($($tail),*);
    };
}

impl_matcher_for_one_of_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
