use std::cell::OnceCell;

use crate::grammar::{
    context::ParserContext,
    error_handler::{ErrorHandler, ParserError},
    parser::{Parser, ParserObjSafe},
};

pub struct Deferred<'ctx, Token, Output> {
    parser: OnceCell<Box<dyn ParserObjSafe<'ctx, Token, Output = Output>>>,
}

impl<'ctx, Token, Output> Deferred<'ctx, Token, Output> {
    fn new() -> Self {
        Self {
            parser: OnceCell::new(),
        }
    }

    fn set_parser<P>(&self, parser: P) -> Result<(), &'static str>
    where
        P: Parser<'ctx, Token, Output = Output> + 'static,
    {
        self.parser
            .set(Box::new(parser))
            .map_err(|_| "Parser has already been set")
    }
}

impl<'ctx, Token, Output> Parser<'ctx, Token> for Deferred<'ctx, Token, Output> {
    type Output = Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        if let Some(parser) = self.parser.get() {
            parser.parse(context, error_handler.to_choice(), pos)
        } else {
            panic!("Deferred parser was not set before parsing")
        }
    }
}

pub fn recursive<'ctx, Token, Output, F, Pars>(parser_fn: F) -> Deferred<'ctx, Token, Output>
where
    F: FnOnce(&Deferred<'ctx, Token, Output>) -> Pars,
    Pars: Parser<'ctx, Token, Output = Output> + 'static,
{
    let deferred = Deferred::new();
    let parser = parser_fn(&deferred);
    deferred.set_parser(parser).expect("Failed to set parser");
    deferred
}
