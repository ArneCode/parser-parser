//! [`as_parser`]: turn a concrete parser type into `impl Parser + Clone` at the boundary.
//!
//! This lives outside [`super::capture`] so opaque-parser helpers stay separate from capture
//! machinery (`Capture`, binders, match results).

use crate::input::Input;
use crate::parser::{Parser, internal::ParserImpl};

/// Wraps a parser so call sites see `impl Parser` instead of a concrete combinator type.
///
/// Used by the [`crate::capture!`] macro and by hand-written grammars (for example the JSON
/// example) to introduce type-opacity boundaries.
#[inline]
pub fn as_parser<'src, Inp, P>(
    parser: P,
) -> impl Parser<'src, Inp, Output = <P as ParserImpl<'src, Inp>>::Output> + Clone
where
    P: Parser<'src, Inp> + Clone,
    Inp: Input<'src>,
{
    parser
}
