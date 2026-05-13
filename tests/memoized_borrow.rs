//! Regression: [`marser::parser::Memoized`] must accept parser outputs that borrow the input (`'src`).

use std::rc::Rc;

use marser::parser::{Parser, ParserCombinator};
use marser_macros::capture;

fn letter_word<'src>() -> impl Parser<'src, &'src str, Output = Rc<&'src str>> + Clone {
    capture!(
        bind_slice!(
            (
                marser::one_of::one_of(('a'..='z', 'A'..='Z')),
                marser::matcher::many(marser::one_of::one_of((
                    'a'..='z',
                    'A'..='Z',
                    '0'..='9',
                ))),
            ),
            slice as &'src str
        ) => slice
    )
    .memoized()
}

#[test]
fn memoized_parser_with_borrowed_output_parse_str() {
    let p = letter_word();
    let (a, errs_a) = p.parse_str("hello").expect("first parse");
    assert!(errs_a.is_empty());
    let (b, errs_b) = p.parse_str("hello").expect("second parse");
    assert!(errs_b.is_empty());
    assert_eq!(*a, "hello");
    assert_eq!(*b, "hello");
}
