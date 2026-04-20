//! Fixed-point and mutually recursive parsers via a cell filled after construction.
//!
//! Use [`recursive`] to obtain a [`Deferred`] handle, build a parser that closes over
//! [`DeferredWeak`], then parse through the strong [`Deferred`].

use std::{
    cell::OnceCell,
    rc::{Rc, Weak},
};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::{Parser, ParserObjSafe},
};

/// Strong handle to a parser installed later; used as the entry point for a recursive grammar.
#[derive(Clone)]
pub struct Deferred<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
{
    parser: Rc<OnceCell<Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>>>,
}

/// Weak back-reference for defining recursive productions without a cycle at construction time.
#[derive(Clone)]
pub struct DeferredWeak<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
{
    parser: Weak<OnceCell<Box<dyn ParserObjSafe<'src, Inp, Output> + 'a>>>,
}

impl<'a, 'src, Inp, Output> Deferred<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
{
    fn new() -> Self {
        Self {
            parser: Rc::new(OnceCell::new()),
        }
    }

    fn set_parser<P>(&self, parser: P) -> Result<(), &'static str>
    where
        P: Parser<'src, Inp, Output = Output> + 'a,
    {
        self.parser
            .set(Box::new(parser))
            .map_err(|_| "Parser has already been set")
    }

    fn clone_weak(&self) -> DeferredWeak<'a, 'src, Inp, Output> {
        DeferredWeak {
            parser: Rc::downgrade(&self.parser),
        }
    }
}

impl<'a, 'src, Inp, Output> super::internal::ParserImpl<'src, Inp>
    for Deferred<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
{
    type Output = Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        if let Some(parser) = self.parser.get() {
            parser.parse(context, error_handler.to_choice(), input)
        } else {
            panic!("Deferred parser was not set before parsing")
        }
    }
}

impl<'a, 'src, Inp, Output> super::internal::ParserImpl<'src, Inp>
    for DeferredWeak<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
{
    type Output = Output;
    const CAN_FAIL: bool = true;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        if let Some(parser) = self.parser.upgrade() {
            if let Some(parser) = parser.get() {
                parser.parse(context, error_handler.to_choice(), input)
            } else {
                panic!("Deferred parser was not set before parsing")
            }
        } else {
            panic!("Deferred parser was dropped before parsing")
        }
    }
}

/// Creates a [`Deferred`] parser: `parser_fn` receives a weak handle and must return the real parser.
pub fn recursive<'a, 'src, Inp, Output, F, Pars>(parser_fn: F) -> Deferred<'a, 'src, Inp, Output>
where
    Inp: Input<'src>,
    F: FnOnce(DeferredWeak<'a, 'src, Inp, Output>) -> Pars,
    Pars: Parser<'src, Inp, Output = Output> + 'a,
{
    let deferred = Deferred::new();
    let parser = parser_fn(deferred.clone_weak());
    deferred.set_parser(parser).expect("Failed to set parser");
    deferred
}
