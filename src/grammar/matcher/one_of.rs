use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    options: Tuple,
    id: usize,
    phantom: PhantomData<(T, N)>,
}

impl<T, N, Tuple> OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
    Tuple: IsCheckable<T> + Matcher<T, Output = N>,
{
    fn new(options: Tuple) -> Self {
        Self {
            options,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

impl<T, N, Tuple> HasId for OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, N, $head, $($tail),*> IsCheckable<T> for OneOfMatcher<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode,
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

        impl<T, N, $head, $($tail),*> Matcher<T> for OneOfMatcher<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode + ?Sized,
            $head: Matcher<T, Output = N> + Grammar<T>,
            $($tail: Matcher<T, Output = N> + Grammar<T>,)*
        {
            type Output = N;

            fn match_pattern(
                &self,
                context: &mut MatcherContext<T, Self::Output>,
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
