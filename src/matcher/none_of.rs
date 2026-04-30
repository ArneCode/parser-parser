use crate::{
    matcher::{AnyToken, NegativeLookahead, negative_lookahead},
    one_of::{OneOf, one_of},
};

#[cfg(feature = "parser-trace")]
#[track_caller]
pub fn none_of<Tuple>(tuple: Tuple) -> (NegativeLookahead<OneOf<Tuple>>, AnyToken) {
    (negative_lookahead(one_of(tuple)), AnyToken)
}

#[cfg(not(feature = "parser-trace"))]
pub fn none_of<Tuple>(tuple: Tuple) -> (NegativeLookahead<OneOf<Tuple>>, AnyToken) {
    (negative_lookahead(one_of(tuple)), AnyToken)
}
