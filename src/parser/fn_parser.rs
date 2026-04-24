use std::marker::PhantomData;

use crate::{
    error::error_handler::{ErrorHandler, ErrorHandlerChoice},
    input::Input,
    parser::{Parser, internal::ParserImpl},
};

pub(crate) struct FnParser<ParseFn, Out> {
    parse_fn: ParseFn,
    _phantom: PhantomData<Out>,
}

impl<ParseFn, Out> std::fmt::Debug for FnParser<ParseFn, Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnParser").finish()
    }
}

impl<ParseFn, Out> Clone for FnParser<ParseFn, Out>
where
    ParseFn: Clone,
{
    fn clone(&self) -> Self {
        Self {
            parse_fn: self.parse_fn.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<ParseFn, Out> FnParser<ParseFn, Out> {
    pub fn new(parse_fn: ParseFn) -> Self {
        Self {
            parse_fn,
            _phantom: PhantomData,
        }
    }
}

pub fn break_type<'src, Pars, Inp>(
    parser: Pars,
) -> FnParser<
    impl for<'ctx, 'eh> Fn(
        &'ctx mut crate::context::ParserContext,
        ErrorHandlerChoice<'eh>,
        &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<Option<Pars::Output>, crate::error::FurthestFailError>,
    Pars::Output,
>
where
    Pars: Parser<'src, Inp>,
    Inp: Input<'src>,
{
    // Identity helper that forces the closure to be inferred with higher-ranked
    // lifetimes for `'ctx` and `'eh`. Without this, closure type inference pins
    // those lifetimes to a single specific lifetime and fails to satisfy the
    // `for<'ctx, 'eh> Fn(...)` bound expected by `FnParser`.
    fn constrain<'src, F, Out, Inp>(f: F) -> F
    where
        F: for<'ctx, 'eh> Fn(
            &'ctx mut crate::context::ParserContext,
            ErrorHandlerChoice<'eh>,
            &mut crate::input::InputStream<'src, Inp>,
        ) -> Result<Option<Out>, crate::error::FurthestFailError>,
        Inp: Input<'src>,
    {
        f
    }

    FnParser::new(constrain(
        move |context, error_handler, input| match error_handler {
            ErrorHandlerChoice::Empty(handler) => parser.parse(context, handler, input),
            ErrorHandlerChoice::Multi(handler) => parser.parse(context, handler, input),
        },
    ))
}

impl<ParseFn, Out> crate::parser::ParserCombinator for FnParser<ParseFn, Out> where ParseFn: Clone {}

impl<'src, ParseFn, Out, Inp> ParserImpl<'src, Inp> for FnParser<ParseFn, Out>
where
    ParseFn: for<'ctx, 'eh> Fn(
            &'ctx mut crate::context::ParserContext,
            ErrorHandlerChoice<'eh>,
            &mut crate::input::InputStream<'src, Inp>,
        ) -> Result<Option<Out>, crate::error::FurthestFailError>
        + Clone,
    Inp: crate::input::Input<'src>,
{
    type Output = Out;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut crate::context::ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut crate::input::InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, crate::error::FurthestFailError> {
        (self.parse_fn)(context, error_handler.to_choice(), input)
    }
}

#[cfg(test)]
mod tests {
    use crate::{error::error_handler::ErrorHandlerChoice, parse, parser::fn_parser::FnParser};

    fn f<'src>(
        context: &mut crate::context::ParserContext,
        empty_handler: ErrorHandlerChoice,
        input: &mut crate::input::InputStream<'src, impl crate::input::Input<'src>>,
    ) -> Result<Option<()>, crate::error::FurthestFailError> {
        Ok(Some(()))
    }

    #[test]
    fn test_fn_parser() {
        let parser = FnParser::new(f);
        let mut context = crate::context::ParserContext::new();
        let result = parse(parser, "abc");
        // assert_eq!(result.unwrap(), Some(()));
    }
}
