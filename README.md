# marser

`marser` is a parser-combinator library centered on explicit matcher composition,
backtracking control, and rich source diagnostics.

## Quick start

```rust
use marser::matcher::many;
use marser::one_of::one_of;
use marser::parser::token_parser::TokenParser;
use marser::parse;
use marser_macros::capture;

let digit = TokenParser::new(|c: &char| c.is_ascii_digit(), |c: &char| *c);
let number = capture!((bind!(&digit, *digits), many(bind!(&digit, *digits))) => {
    digits.into_iter().collect::<String>()
});

let (parsed, _warnings) = parse(number, "1234").unwrap();
assert_eq!(parsed, "1234");
```

## Example

A full JSON grammar example is available at `examples/json.rs` and can be run with:

```bash
cargo run --example json
```

## License

This project is licensed under the MIT License.
