use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner,
    },
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
        impl<'a, 'ctx, Runner, $head, $($tail),*> CanImplMatchWithRunner<Runner> for OneOfMatcher<($head, $($tail,)*)>
        where
            Runner: MatchRunner<'a, 'ctx>,
            $head: CanMatchWithRunner<Runner>,
            $($tail: CanMatchWithRunner<Runner>,)*
        {
            fn impl_match_with_runner(&self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<bool, ParserError> {
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

        impl<$head, $($tail),*> DoImplMatchWithNoMoemoizeBacktrackingRunner for OneOfMatcher<($head, $($tail,)*)>
        where
            $head: DoImplMatchWithNoMoemoizeBacktrackingRunner,
            $($tail: DoImplMatchWithNoMoemoizeBacktrackingRunner,)*
        {
        }

        impl_matcher_for_one_of_tuples!($($tail),*);
    };
}

impl_matcher_for_one_of_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
