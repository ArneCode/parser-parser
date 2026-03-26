use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, get_next_id, matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};
pub struct OneOfMatcher<T, MContext, Tuple> {
    options: Tuple,
    id: usize,
    phantom: PhantomData<(T, MContext)>,
}

impl<T, MContext, Tuple> OneOfMatcher<T, MContext, Tuple> {
    pub fn new(options: Tuple) -> Self {
        Self {
            options,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

impl<T, MContext, Tuple> HasId for OneOfMatcher<T, MContext, Tuple> {
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, MContext, $head, $($tail),*> IsCheckable<T> for OneOfMatcher<T, MContext,($head, $($tail,)*)>
        where
            $head: Grammar<T>,
            $($tail: Grammar<T>,)*
        {
            fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check(context, pos) {
                    return true;
                }

                $(
                    if $tail.check(context, pos) {
                        return true;
                    }
                )*

                false
            }
        }

        impl<T, MContext, $head, $($tail),*> Matcher<T, MContext> for OneOfMatcher<T, MContext,($head, $($tail,)*)>
        where
            MContext: Deref<Target = ParserContext<T>>,
            $head: Matcher<T, MContext> + Grammar<T>,
            $($tail: Matcher<T, MContext> + Grammar<T>,)*
        {

            fn match_pattern(
                &self,
                context: &mut MContext,
                pos: &mut usize,
            ) -> Result<(), String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check(context, pos) {
                    return $head.match_pattern(context, pos);
                }

                $(
                    if $tail.check(context, pos) {
                        return $tail.match_pattern(context, pos);
                    }
                )*

                Err(format!("None of the options matched at position {}", pos))
            }
        }

        impl_matcher_for_one_of_tuples!($($tail),*);
    };
}

impl_matcher_for_one_of_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
