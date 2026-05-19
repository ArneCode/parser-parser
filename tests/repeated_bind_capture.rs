//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.
//!
//! Repeated compatible `*name` binds in one `capture!` (regression for bind registry merging).

use marser::capture;
use marser::one_of::one_of;
use marser::parser::Parser;

#[test]
fn repeated_compatible_multiple_binds_parse_str() {
    let digit = one_of(('a', 'b'));
    let p = capture!({
        (
            bind!(digit.clone(), *xs),
            bind!(digit.clone(), *xs)
        )
    } => xs);
    let (out, errs) = p.parse_str("ab").expect("parse");
    assert!(errs.is_empty());
    assert_eq!(out, vec!['a', 'b']);
}
