use marser::{parse, parser::token_parser::TokenParser};

#[test]
fn token_parser_smoke_test() {
    let parser = TokenParser::new(|c: &char| c.is_ascii_digit(), |c: &char| *c);
    let (result, warnings) = parse(parser, "7").expect("single-digit parse should succeed");
    assert_eq!(result, '7');
    assert!(warnings.is_empty());
}

#[test]
fn token_parser_rejects_non_digit() {
    let parser = TokenParser::new(|c: &char| c.is_ascii_digit(), |c: &char| *c);
    assert!(parse(parser, "x").is_err());
}
