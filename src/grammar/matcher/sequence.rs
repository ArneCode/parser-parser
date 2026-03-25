use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    elements: Tuple,
    id: usize,
    phantom: PhantomData<(T, N)>,
}
impl<T, N, Tuple> Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    fn new(elements: Tuple) -> Self {
        Self {
            elements,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

pub fn seq<T, N, Tuple>(elements: Tuple) -> Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    Sequence::new(elements)
}

impl<T, N, Tuple> HasId for Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, N, $head, $($tail),*> IsCheckable<T> for Sequence<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode,
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
        impl<T, N, $head, $($tail),*> Matcher<T> for Sequence<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode + ?Sized,
            $head: Matcher<T, Output = N>,
            $($tail: Matcher<T, Output = N>,)*
        {
            type Output = N;

            fn match_pattern(
                &self,
                context: &mut MatcherContext<T, N>,
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
