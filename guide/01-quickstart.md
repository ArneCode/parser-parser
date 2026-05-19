Dependencies, parser vs matcher, and your first parser.

<div style="background-color: #fff8e1; border-left: 4px solid #f9a825; padding: 0.75em 1em; margin: 1em 0;">

**AI assistance:** This chapter was drafted with AI assistance while the library is still young. The guide is expected to improve over time as APIs and examples stabilize. If anything looks wrong or confusing, please [report it on GitHub](https://github.com/ArneCode/marser/issues/new).

</div>

# Quickstart

This section gets you from zero to a tiny working parser.

## 1) Add dependency

In your `Cargo.toml`:

```toml
[dependencies]
marser = { version = "0.1.0", features = ["annotate-snippets"] }
```

The **`annotate-snippets`** feature matches how this repository runs examples (`ParserError::eprint`). You can omit it if you do not need the optional [annotate-snippets](https://docs.rs/annotate-snippets) dependency or the `eprint` / `write` helpers.

For local development of this repository, you can run examples directly:

```bash
cargo run -p marser --features annotate-snippets --example json -- tests/data/json1.json
```

## 2) Mental model: parser vs matcher

- A `Matcher` checks whether input fits a pattern at the current position.
- A `Parser` produces a typed output value when matching succeeds.
- `capture!` combines matcher patterns and then constructs parser output.
- Most grammar building blocks (sequence, lookahead, repetition) are matcher-level.
- The final rule you run with `parser.parse_str(...)` (or the thin alias `marser::parse(parser, ...)`) is a parser.

If you remember one thing: matchers describe structure, parsers return values.

## 3) First parser: literals and ordered choice

This example shows **ordered choice** (`one_of`) over a few keyword parsers, each built with `capture!`. Every branch returns the same `Value` type, which is the usual pattern for alternatives.

```rust
use marser::capture;
use marser::one_of::one_of;
use marser::parser::Parser;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Bool(bool),
}

fn grammar<'src>() -> impl Parser<'src, &'src str, Output = Value> + Clone {
    one_of((
        capture!(("null") => Value::Null),
        capture!(("true") => Value::Bool(true)),
        capture!(("false") => Value::Bool(false)),
    ))
}

fn main() {
    let parser = grammar();
    let (value, errors) = parser.parse_str("false").unwrap();
    assert_eq!(value, Value::Bool(false));
    assert!(errors.is_empty());
}
```

Why this matters when evaluating `marser`: the **grammar reads like a table of alternatives**, and `capture!` stays the bridge from matcher-shaped input to your AST or enum.

## 4) What this example already shows

- **Ordered choice stays local**: every branch in `one_of((...))` produces the same `Value` type, so the grammar reads like a direct list of alternatives.
- **`capture!` is the value boundary**: the matcher side says what to consume, and the expression after `=>` says what semantic value to build.
- **Whole-input parse is the default**: `parse_str("false")` succeeds, but `parse_str("false trailing")` fails because `marser` wraps the parser with an EOF check.

If you want the next step after this page, there are two good directions:

- continue to [Core Concepts](crate::guide::core_concepts) if you want the mental model first
- jump to [Build a Simple JSON Parser](crate::guide::worked_json_example) if you prefer learning from a full grammar

## 5) Common mistakes

- **Confusing matcher and parser output**: matcher checks structure; parser returns your enum/AST.
- **Forgetting full-input parse behavior**: `Parser::parse_str` / `marser::parse` expect no trailing tokens.
- **Starting too big**: begin with one literal rule, then add alternatives and recursion.

Next: [Core Concepts](crate::guide::core_concepts)
