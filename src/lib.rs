#![doc = include_str!("../README.md")]
#![allow(private_bounds)]
extern crate self as marser;

pub(crate) mod context;

pub mod error;
pub mod label;
pub mod matcher;
pub mod parser;

use marser_macros::capture;
use std::rc::Rc;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, ParserError, error_handler::EmptyErrorHandler},
    matcher::{
        any_token::AnyToken, commit_matcher::commit_on, negative_lookahead::negative_lookahead,
    },
    parser::{Parser, internal::ParserImpl},
};

pub fn parse<Pars>(
    parser: Pars,
    src: &str,
) -> Result<(Pars::Output, Vec<ParserError>), FurthestFailError>
where
    Pars: Parser<char>,
{
    let tokens: Vec<char> = src.chars().collect();
    let mut error_handler = EmptyErrorHandler;
    let mut context = ParserContext::new(&tokens);
    let mut pos = 0;
    let parser = Rc::new(parser);

    let parser = capture!(
        commit_on((), (
            bind!(parser.clone(), result),
            negative_lookahead(AnyToken),
        )) => result
    );
    let result = parser
        .parse(&mut context, &mut error_handler, &mut pos)?
        .unwrap();
    Ok((result, context.get_errors()))
}
