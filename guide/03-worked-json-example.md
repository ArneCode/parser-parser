# Build a Simple JSON Parser

This page shows how to build your own JSON parser with `marser`.
It is intentionally simpler than production JSON: it supports

- `null`
- booleans (`true`/`false`)
- integers (no exponent handling)
- strings without escape sequences
- arrays
- objects

The approach is incremental: each section has a full runnable block that contains
everything from the previous section plus one new concept.

## 1) Start with `null` only

Before parsing complex JSON, we establish the basic shape:

- define one AST enum we can keep extending
- parse a single valid JSON literal (`null`)
- include whitespace handling early so later rules are easier to compose

Why start this way: it gives a known-good baseline where parser output, error
collection, and whitespace handling are all visible in one tiny example.

Mini idea (illustrative only):

```rust,ignore
// "token parser" idea:
// optional whitespace + "null" + optional whitespace => JsonValue::Null
```

```rust
use marser::one_of::one_of;
use marser::parser::{Parser, deferred::recursive};
use marser_macros::capture;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> {
    recursive(|_value| {
        let ws = Rc::new(marser::matcher::multiple::many(one_of((' ', '\n', '\r', '\t'))));
        capture!((ws.clone(), "null", ws.clone()) => JsonValue::Null)
    })
}

let (value, errors) = marser::parse(json(), "null").unwrap();
assert_eq!(value, JsonValue::Null);
assert!(errors.is_empty());
```

## 2) Grow it: add booleans, numbers, and strings

Now we add scalar values. The key design decisions are:

- **ordered alternatives** with `one_of((...))`: try string, number, boolean, null
- **string re-use**: parse raw string first, then map it to `JsonValue::String`
- **typed slices** in `bind_slice!`: keeping `&'src str` avoids inference trouble

Why this order helps: scalar rules are independent and easy to debug before we
introduce recursion for arrays/objects.

Mini idea (illustrative only):

```rust,ignore
// number shape in this tutorial:
// optional('-') + one_or_more('0'..='9')
```

```rust
use marser::one_of::one_of;
use marser::parser::{Parser, ParserCombinator, deferred::recursive};
use marser_macros::capture;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> {
    recursive(|_value| {
        let ws = Rc::new(marser::matcher::multiple::many(one_of((' ', '\n', '\r', '\t'))));
        let null = capture!(("null", ws.clone()) => JsonValue::Null);
        let t = capture!(("true", ws.clone()) => JsonValue::Bool(true));
        let f = capture!(("false", ws.clone()) => JsonValue::Bool(false));
        let boolean = one_of((t, f));

        let number = capture!((
            marser::matcher::optional::optional('-'),
            bind_slice!(marser::matcher::one_or_more::one_or_more('0'..='9'), n as &'src str),
            ws.clone()
        ) => JsonValue::Number(n.parse().unwrap()));

        let str_char = one_of(('a'..='z', 'A'..='Z', '0'..='9', ' ', '_', '-'));
        let raw_string = Rc::new(capture!((
            '"',
            bind_slice!(marser::matcher::multiple::many(str_char), s as &'src str),
            '"',
            ws.clone()
        ) => s.to_string()));
        let string = raw_string.map_output(JsonValue::String);

        capture!((
            ws.clone(),
            bind!(one_of((string, number, boolean, null)), out),
            ws.clone()
        ) => out)
    })
}

let (value, errors) = marser::parse(json(), " 123 ").unwrap();
assert_eq!(value, JsonValue::Number(123));
assert!(errors.is_empty());
```

## 3) Grow it again: add arrays and objects

This is where recursion matters:

- arrays contain `value` elements
- object values are also `value`
- therefore `json()` must be defined with `recursive(...)`

The object rule is built from a small `key_value` parser so we do not duplicate
`"key": value` logic inside the list machinery.

Mini idea (illustrative only):

```rust,ignore
// object shape:
// '{' [ key_value (',' key_value)* ] '}'
```

```rust
use marser::one_of::one_of;
use marser::parser::{Parser, ParserCombinator, deferred::recursive};
use marser_macros::capture;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> {
    recursive(|value| {
        let ws = Rc::new(marser::matcher::multiple::many(one_of((' ', '\n', '\r', '\t'))));
        let null = capture!(("null", ws.clone()) => JsonValue::Null);
        let t = capture!(("true", ws.clone()) => JsonValue::Bool(true));
        let f = capture!(("false", ws.clone()) => JsonValue::Bool(false));
        let boolean = one_of((t, f));

        let number = capture!((
            marser::matcher::optional::optional('-'),
            bind_slice!(marser::matcher::one_or_more::one_or_more('0'..='9'), n as &'src str),
            ws.clone()
        ) => JsonValue::Number(n.parse().unwrap()));

        let str_char = one_of(('a'..='z', 'A'..='Z', '0'..='9', ' ', '_', '-'));
        let raw_string = Rc::new(capture!((
            '"',
            bind_slice!(marser::matcher::multiple::many(str_char), s as &'src str),
            '"',
            ws.clone()
        ) => s.to_string()));
        let string = raw_string.clone().map_output(JsonValue::String);

        let key_value = Rc::new(capture!((
            bind!(raw_string.clone(), key),
            ':',
            ws.clone(),
            bind!(value.clone(), val)
        ) => (key, val)));

        let array = capture!((
            '[',
            ws.clone(),
            marser::matcher::optional::optional((
                bind!(value.clone(), *items),
                marser::matcher::multiple::many((
                    ',',
                    ws.clone(),
                    bind!(value.clone(), *items)
                ))
            )),
            ']',
            ws.clone()
        ) => JsonValue::Array(items));

        let object = capture!((
            '{',
            ws.clone(),
            marser::matcher::optional::optional((
                bind!(key_value.clone(), *entries),
                marser::matcher::multiple::many((
                    ',',
                    ws.clone(),
                    bind!(key_value.clone(), *entries)
                ))
            )),
            '}',
            ws.clone()
        ) => JsonValue::Object(entries));

        capture!((
            ws.clone(),
            bind!(one_of((object, array, string, number, boolean, null)), out),
            ws.clone()
        ) => out)
    })
}

let src = r#"{"ok": true, "items": [1, 2, 3]}"#;
let (value, errors) = marser::parse(json(), src).unwrap();
assert!(errors.is_empty());
assert!(matches!(value, JsonValue::Object(_)));
```

At this point, you have a complete, recursive JSON parser with a small grammar.
The string rule is intentionally simple and does not handle escapes.

## 4) What to improve next

- **Escaped strings**: support `\"`, `\\`, `\n`, and unicode escapes.
- **Full numbers**: add fractional and exponent forms (`1.2`, `1e3`).
- **Recovery**: add `recover_with(...)` so malformed sections become fallback nodes.
- **Diagnostics**: attach richer labels/notes using `add_error_info(...)`.

After this page, continue with [Errors and Recovery](crate::guide::errors_and_recovery).
