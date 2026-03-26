use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct Sequence<T, MContext, Tuple> {
    elements: Tuple,
    id: usize,
    phantom: PhantomData<(T, MContext)>,
}
impl<T, MContext, Tuple> Sequence<T, MContext, Tuple> {
    fn new(elements: Tuple) -> Self {
        Self {
            elements,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

pub fn seq<T, MContext, Tuple>(elements: Tuple) -> Sequence<T, MContext, Tuple> {
    Sequence::new(elements)
}

impl<T, MContext, Tuple> HasId for Sequence<T, MContext, Tuple> {
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, MContext, $head, $($tail),*> IsCheckable<T> for Sequence<T, MContext,($head, $($tail,)*)>
        where
            $head: Grammar<T>,
            $($tail: Grammar<T>,)*
        {
            fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.elements;

                if !$head.check(context, pos) {
                    return false;
                }

                $(
                    if !$tail.check(context, pos) {
                        return false;
                    }
                )*

                true
            }
        }
        impl<T, MContext, $head, $($tail),*> Matcher<T, MContext> for Sequence<T, MContext,($head, $($tail,)*)>
        where
            $head: Matcher<T, MContext>,
            $($tail: Matcher<T, MContext>,)*
        {

            fn match_pattern(
                &self,
                context: &mut MContext,
                pos: &mut usize,
            ) -> Result<(), String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.elements;

                $head.match_pattern(context, pos)?;

                $(
                $tail.match_pattern(context, pos)?;
                )*

                Ok(())
            }
        }

        impl_matcher_for_seq_tuples!($($tail),*);
    };
}

impl_matcher_for_seq_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
