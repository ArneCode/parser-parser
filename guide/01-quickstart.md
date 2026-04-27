# Quickstart

This section gets you from zero to a tiny working parser.

## 1) Add dependency

In your `Cargo.toml`:

```toml
[dependencies]
marser = "0.1.0"
```

For local development of this repository, you can run examples directly:

```bash
cargo run --example json -- tests/data/json1.json
```

## 2) Mental model: parser vs matcher

- A `Matcher` checks whether input fits a pattern at the current position.
- A `Parser` produces a typed output value when matching succeeds.
- `capture!` combines matcher patterns and then constructs parser output.
- Most grammar building blocks (sequence, lookahead, repetition) are matcher-level.
- The final rule you run with `marser::parse(...)` is a parser.

If you remember one thing: matchers describe structure, parsers return values.

## 3) First tiny parser

```rust,ignore
use marser::capture;
use marser::parser::Parser;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
}

fn grammar<'src>() -> impl Parser<'src, &'src str, Output = Value> {
    capture!(("null") => Value::Null)
}

fn main() {
    let input = "null";
    let parser = grammar();
    let (value, errors) = marser::parse(parser, input).unwrap();
    assert_eq!(value, Value::Null);
    assert!(errors.is_empty());
}
```

## 4) Common mistakes

- **Confusing matcher and parser output**: matcher checks structure; parser returns your enum/AST.
- **Forgetting full-input parse behavior**: `marser::parse` expects no trailing tokens.
- **Starting too big**: begin with one literal rule, then add alternatives and recursion.

Next: [Core Concepts](crate::guide::core_concepts)
