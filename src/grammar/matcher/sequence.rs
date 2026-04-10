use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner,
    },
};
pub struct Sequence<Tuple> {
    elements: Tuple,
}
impl<Tuple> Sequence<Tuple> {
    fn new(elements: Tuple) -> Self {
        Self { elements }
    }
}

pub fn seq<Tuple>(elements: Tuple) -> Sequence<Tuple> {
    Sequence::new(elements)
}

macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<'a, 'ctx, Runner, $head, $($tail),*> CanImplMatchWithRunner<Runner> for Sequence<($head, $($tail,)*)>
        where
            $head: CanMatchWithRunner<Runner>,
            $($tail: CanMatchWithRunner<Runner>,)*
            Runner: MatchRunner<'a, 'ctx>,
        {
            fn impl_match_with_runner(&self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<bool, ParserError> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.elements;

                if !runner.run_match($head, error_handler, pos)? {
                    return Ok(false);
                }

                $(
                    if !runner.run_match($tail, error_handler, pos)? {
                        return Ok(false);
                    }
                )*

                Ok(true)
            }
        }

        impl<$head, $($tail),*> DoImplMatchWithNoMoemoizeBacktrackingRunner for Sequence<($head, $($tail,)*)>
        where
            $head: DoImplMatchWithNoMoemoizeBacktrackingRunner,
            $($tail: DoImplMatchWithNoMoemoizeBacktrackingRunner,)*
        {
        }

        impl_matcher_for_seq_tuples!($($tail),*);
    };
}

impl_matcher_for_seq_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
