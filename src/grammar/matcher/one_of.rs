use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
    parser::Parser,
};
pub struct OneOf<Tuple> {
    options: Tuple,
}

impl<Tuple> OneOf<Tuple> {
    pub fn new(options: Tuple) -> Self {
        Self { options }
    }
}

pub fn one_of<Options>(options: Options) -> OneOf<Options> {
    OneOf::new(options)
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<Token, MRes, $head, $($tail),*> Matcher<Token, MRes> for OneOf<($head, $($tail,)*)>
        where
            $head: Matcher<Token, MRes>,
            $($tail: Matcher<Token, MRes>,)*
        {
            const CAN_MATCH_DIRECTLY: bool = $head::CAN_MATCH_DIRECTLY  $(&& $tail::CAN_MATCH_DIRECTLY)*;
            const HAS_PROPERTY: bool = $head::HAS_PROPERTY  $(|| $tail::HAS_PROPERTY)*;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;

            fn match_with_runner<'a, 'ctx, Runner>(&'a self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<bool, ParserError>
            where
                Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
                'ctx: 'a,
            {
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
        impl<Token, Output, $head, $($tail),*> Parser<Token> for OneOf<($head, $($tail,)*)>
        where
            $head: Parser<Token, Output = Output>,
            $($tail: Parser<Token, Output = Output>,)*
        {
            type Output = Output;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(&& $tail::CAN_FAIL)*;
            fn parse(&self, context: &mut ParserContext<Token>, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<Option<Output>, ParserError> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                // if $head.check_no_advance(context, pos) {
                //     return $head.parse(context, pos);
                // }
                if let Some(output) = $head.parse(context, error_handler, pos)? {
                    return Ok(Some(output));
                }

                // $(
                //     if $tail.check_no_advance(context, pos) {
                //         return $tail.parse(context, pos);
                //     }
                // )*
                $(
                    if let Some(output) = $tail.parse(context, error_handler, pos)? {
                        return Ok(Some(output));
                    }
                )*

                Ok(None)
            }
        }
        impl_matcher_for_one_of_tuples!($($tail),*);
    };
}

impl_matcher_for_one_of_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
