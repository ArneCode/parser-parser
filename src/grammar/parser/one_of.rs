use crate::grammar::{
    Grammar, HasId, IsCheckable, context::ParserContext, error_handler::ErrorHandler, get_next_id,
    label::MaybeLabel, parser::Parser,
};
pub struct OneOfParser<Tuple> {
    options: Tuple,
    id: usize,
}

impl<Tuple> OneOfParser<Tuple> {
    pub fn new(options: Tuple) -> Self {
        Self {
            options,
            id: get_next_id(),
        }
    }
}

impl<Tuple> HasId for OneOfParser<Tuple> {
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_parser_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<Token, $head, $($tail),*> IsCheckable<Token> for OneOfParser<($head, $($tail,)*)>
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

        impl<Token, Output, $head, $($tail),*> Parser<Token> for OneOfParser<($head, $($tail,)*)>
        where
            $head: Parser<Token, Output = Output> + Grammar<Token>,
            $($tail: Parser<Token, Output = Output> + Grammar<Token>,)*
        {
            type Output = Output;
            fn parse(&self, context: &mut ParserContext<Token, impl ErrorHandler>, pos: &mut usize) -> Result<Output, String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check_no_advance(context, pos) {
                    return $head.parse(context, pos);
                }

                $(
                    if $tail.check_no_advance(context, pos) {
                        return $tail.parse(context, pos);
                    }
                )*

                Err("No options matched".to_string())
            }
        }
        impl_parser_for_one_of_tuples!($($tail),*);
    };
}

impl_parser_for_one_of_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

impl<Label, Tuple> MaybeLabel<Label> for OneOfParser<Tuple> {}
