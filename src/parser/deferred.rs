use std::{
    cell::OnceCell,
    rc::{Rc, Weak},
};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    parser::{Parser, ParserObjSafe},
};

#[derive(Clone)]
pub struct Deferred<'a, Token, Output> {
    parser: Rc<OnceCell<Box<dyn ParserObjSafe<Token, Output = Output> + 'a>>>,
}
#[derive(Clone)]
pub struct DeferredWeak<'a, Token, Output> {
    parser: Weak<OnceCell<Box<dyn ParserObjSafe<Token, Output = Output> + 'a>>>,
}

impl<'a, Token, Output> Deferred<'a, Token, Output> {
    fn new() -> Self {
        Self {
            parser: Rc::new(OnceCell::new()),
        }
    }

    fn set_parser<P>(&self, parser: P) -> Result<(), &'static str>
    where
        P: Parser<Token, Output = Output> + 'a,
    {
        self.parser
            .set(Box::new(parser))
            .map_err(|_| "Parser has already been set")
    }

    fn clone_weak(&self) -> DeferredWeak<'a, Token, Output> {
        DeferredWeak {
            parser: Rc::downgrade(&self.parser),
        }
    }
}

impl<'a, Token, Output> super::internal::ParserImpl<Token> for Deferred<'a, Token, Output> {
    type Output = Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext<Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        if let Some(parser) = self.parser.get() {
            parser.parse(context, error_handler.to_choice(), pos)
        } else {
            panic!("Deferred parser was not set before parsing")
        }
    }
}

impl<'a, Token, Output> super::internal::ParserImpl<Token> for DeferredWeak<'a, Token, Output> {
    type Output = Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext<Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        if let Some(parser) = self.parser.upgrade() {
            if let Some(parser) = parser.get() {
                parser.parse(context, error_handler.to_choice(), pos)
            } else {
                panic!("Deferred parser was not set before parsing")
            }
        } else {
            panic!("Deferred parser was dropped before parsing")
        }
    }
}

pub fn recursive<'a, 'ctx, Token, Output, F, Pars>(parser_fn: F) -> Deferred<'a, Token, Output>
where
    F: FnOnce(DeferredWeak<'a, Token, Output>) -> Pars,
    Pars: Parser<Token, Output = Output> + 'a,
{
    let deferred = Deferred::new();
    let parser = parser_fn(deferred.clone_weak());
    deferred.set_parser(parser).expect("Failed to set parser");
    deferred
}
