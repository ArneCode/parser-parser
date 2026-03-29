use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
};
use std::{marker::PhantomData, ops::Deref};
pub struct OneOfMatcher<Tuple> {
    options: Tuple,
    id: usize,
}

impl<Tuple> OneOfMatcher<Tuple> {
    pub fn new(options: Tuple) -> Self {
        Self {
            options,
            id: get_next_id(),
        }
    }
}

impl<Tuple> HasId for OneOfMatcher<Tuple> {
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<Token, $head, $($tail),*> IsCheckable<Token> for OneOfMatcher<($head, $($tail,)*)>
        where
            $head: Grammar<Token>,
            $($tail: Grammar<Token>,)*
        {
            fn calc_check(&self, context: &mut ParserContext<Token, impl ErrorHandler>, pos: &mut usize) -> bool {

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

        impl<Token, MRes, $head, $($tail),*> Matcher<Token, MRes> for OneOfMatcher<($head, $($tail,)*)>
        where
            $head: Matcher<Token, MRes> + Grammar<Token>,
            $($tail: Matcher<Token, MRes> + Grammar<Token>,)*
        {

            fn match_pattern(
                &self,
                context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
                pos: &mut usize,
            ) -> Result<(), String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check_no_advance(context.parser_context, pos) {
                    return $head.match_pattern(context, pos);
                }

                $(
                    if $tail.check_no_advance(context.parser_context, pos) {
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
