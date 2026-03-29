use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
};
pub struct Sequence<Tuple> {
    elements: Tuple,
    id: usize,
}
impl<Tuple> Sequence<Tuple> {
    fn new(elements: Tuple) -> Self {
        Self {
            elements,
            id: get_next_id(),
        }
    }
}

pub fn seq<Tuple>(elements: Tuple) -> Sequence<Tuple> {
    Sequence::new(elements)
}

impl<Tuple> HasId for Sequence<Tuple> {
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<Token, $head, $($tail),*> IsCheckable<Token> for Sequence<($head, $($tail,)*)>
        where
            $head: Grammar<Token>,
            $($tail: Grammar<Token>,)*
        {
            fn calc_check(&self, context: &mut ParserContext<Token, impl ErrorHandler>, pos: &mut usize) -> bool {

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
        impl<Token, MRes, $head, $($tail),*> Matcher<Token, MRes> for Sequence<($head, $($tail,)*)>
        where
            $head: Matcher<Token, MRes>,
            $($tail: Matcher<Token, MRes>,)*
        {

            fn match_pattern(
                &self,
                context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
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
