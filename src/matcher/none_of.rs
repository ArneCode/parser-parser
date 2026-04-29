use crate::{
    matcher::{AnyToken, NegativeLookahead, negative_lookahead},
    one_of::{OneOf, one_of},
};

pub fn none_of<Tuple>(tuple: Tuple) -> (NegativeLookahead<OneOf<Tuple>>, AnyToken) {
    (negative_lookahead(one_of(tuple)), AnyToken)
}
