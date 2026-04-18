# Marser Grammar Tutorial

This tutorial shows how to use this library to build grammars with parser combinators.

It is written against the current API in this repository (not a generic parser-combinator API).

---

## 1) Core mental model

You build grammars from small parts:

- **Parsers**: implement `Parser<Token, Output = T>`
- **Matchers**: lower-level building blocks used by `capture!`
- **Combinators**: `one_of`, `many`, `optional`, `one_or_more`, `commit_on`, lookaheads
- **Capture macros**: collect matched values into named variables (`bind!`, `bind_span!`)

The usual top-level flow is:

1. Build a parser value.
2. Call `parse(&parser, input_str)`.
3. Get `Result<(output, extra_errors), ParserError>`.

---

## 2) Minimal parser example

```rust
use marser_macros::capture;
use marser::{
    matcher::{one_of::one_of, multiple::many},
    parser::token_parser::TokenParser,
};

// Parse a simple identifier: [a-zA-Z][a-zA-Z0-9_]*
let letter = TokenParser::new(|c: &char| c.is_alphabetic(), |c: &char| *c);
let tail = TokenParser::new(|c: &char| c.is_alphanumeric() || *c == '_', |c: &char| *c);

let ident = capture!(
    (
        bind!(letter, *chars),
        many(bind!(tail, *chars))
    ) => {
        chars.into_iter().collect::<String>()
    }
);
```

Key idea: `capture!` gives you a typed output from a grammar expression.

---

## 3) `capture!` and `bind!` in practice

The most important pattern is:

```rust
capture!(
    ( ... bind!(some_parser, target) ... ) => { /* build output */ }
)
```

`bind!` supports 3 capture kinds:

- `bind!(p, name)` -> single capture (`Option<_>` behind the scenes)
- `bind!(p, *names)` -> repeated capture (`Vec<_>`)
- `bind!(p, ?name)` -> optional capture (`Option<_>`)

You can also capture spans:

- `bind!(p, value_name, span_name)`
- `bind_span!(p, span_name)` (span only)

Spans are `(usize, usize)` in token indices.

---

## 4) Useful combinators

From `marser::matcher`:

- `one_of((a, b, c))`: alternatives
- `many(x)`: zero or more
- `one_or_more(x)`: one or more
- `optional(x)`: optional
- `commit_on(start, then)`: commit once `start` matched; on later failure returns structured error
- `negative_lookahead(x)`, `positive_lookahead(x)`: lookaheads

From labels:

- `.with_label("...")` on parser/matcher pieces for better expected-token errors.

---

## 5) Building recursive grammars

Use `deferred::recursive` for self-referential grammars (arrays/objects/expressions):

```rust
use std::rc::Rc;
use marser::parser::deferred::recursive;

let grammar = recursive(|expr| {
    let expr = Rc::new(expr);
    // build terms that can reference expr.clone()
    // return a parser for one expression node
    one_of((/* ... */))
});
```

For recursive JSON-like structures, this is the standard pattern used in `examples/json.rs`.

---

## 6) Running a grammar against input

Use the helper in `marser::parse`:

```rust
use marser::parse;

let (value, non_fatal_errors) = parse(&my_grammar, input)?;
```

This helper also enforces full input consumption (via lookahead) so trailing garbage is rejected.

---

## 7) Error handling and diagnostics

You have:

- `ParserError` with span + expected labels
- `err.eprint_ariadne(file, source)` for rich terminal diagnostics
- `add_error_info(...)` to attach contextual annotations to failures

Best practice:

- Add `.with_label(...)` to meaningful grammar boundaries (e.g., `"]"`, `"key-value pair"`, `"element"`).
- Use `commit_on` where you want "after this point, this branch must complete".

---

## 8) Testing strategy

### Unit tests

Write focused tests around small parsers (numbers, strings, objects).

### JSONTestSuite (already integrated here)

Current repo has helper tests/scripts:

- `cargo test test_standard_suite -- --nocapture`
- `cargo test test_standard_suite_single_file_from_env -- --nocapture` (with `JSONSUITE_FILE=...`)
- `python3 tests/run_jsonsuite_single.py <file> --mode debug|release`
- `python3 tests/run_jsonsuite_matrix.py --mode both`
- `python3 tests/find_min_stack.py <files...> --mode release`

For very deep recursive cases, you may need:

- `RUST_MIN_STACK=<bytes>`

---

## 9) Common pitfalls

- **Panic vs parse error**: prefer returning `Err(ParserError)`; panics should indicate internal bugs.
- **Unicode escapes**: JSON strings require proper handling of `\\uXXXX`.
- **Control chars in strings**: raw U+0000..U+001F are invalid unless escaped.
- **Deep recursion**: highly nested inputs can stack-overflow in recursive descent parsers.

---

## 10) Suggested workflow for new grammar features

1. Add a focused parser (e.g., number exponent).
2. Add `capture!` output shaping.
3. Label boundaries with `.with_label(...)`.
4. Add small unit tests.
5. Run JSONTestSuite single-file on relevant cases.
6. Run matrix in release and debug to catch regressions.

---

## 11) Quick reference snippets

### Collect many tokens into `String`

```rust
capture!(
    (bind!(token_parser, *chars), many(bind!(token_parser, *chars))) => {
        chars.into_iter().collect::<String>()
    }
)
```

### Optional item

```rust
capture!(
    optional(bind!(item_parser, ?item)) => {
        item
    }
)
```

### Comma-separated list

```rust
capture!(
    (
        optional(bind!(elem.clone(), *items)),
        many((',', bind!(elem.clone(), *items)))
    ) => {
        items
    }
)
```

---
