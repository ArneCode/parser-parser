Build a simple JSON parser in incremental steps.

<div style="background-color: #fff8e1; border-left: 4px solid #f9a825; padding: 0.75em 1em; margin: 1em 0;">

**AI assistance:** This chapter was drafted with AI assistance while the library is still young. The guide is expected to improve over time as APIs and examples stabilize. If anything looks wrong or confusing, please [report it on GitHub](https://github.com/ArneCode/marser/issues/new).

</div>

# Build a Simple JSON Parser

This page shows how to build a small JSON parser with `marser`.
It intentionally supports only a useful subset:

- `null`
- booleans (`true` / `false`)
- integers (including a leading `-`, but no fraction or exponent)
- quoted strings without escape sequences
- arrays
- objects

The approach is incremental: each section has a full runnable block that contains
everything from the previous section plus one new concept.

This tutorial intentionally stays **smaller than the repository's production
example** in `examples/json/grammar.rs` (CLI in `examples/json/main.rs`). Use this page to learn the grammar shape and
the `capture!` patterns; use that example for richer numbers, string
escapes, recovery, diagnostics, and optional tracing.

## 1) Start with `null` only

Before parsing nested JSON, establish the basic shape:

- define an AST enum we can keep extending
- parse one valid JSON literal
- handle surrounding whitespace from the start

We do **not** need recursion yet. Keeping the first version non-recursive makes
the value flow easier to see.

Mini idea (illustrative only):

```text
// optional whitespace + "null" + optional whitespace
```

```rust
use marser::capture;
use marser::matcher::multiple::many;
use marser::one_of::one_of;
use marser::parser::Parser;
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

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> + Clone {
    let ws = Rc::new(many(one_of((' ', '\n', '\r', '\t'))));
    let null = capture!("null" => JsonValue::Null);

    capture!((
        ws.clone(),
        bind!(null, value),
        ws.clone()
    ) => value)
}

let (value, errors) = json().parse_str("  null\t").unwrap();
assert_eq!(value, JsonValue::Null);
assert!(errors.is_empty());
```

This first version already shows a helpful pattern: keep `ws` as a reusable
matcher, then wrap the real value parser with it.

## 2) Grow it: add booleans, numbers, and strings

Now add scalar values. The key ideas are:

- **ordered alternatives** with `one_of((...))`
- **`bind_slice!` for numbers** so the matched text can be parsed directly
- **a small raw-string parser** that we map into `JsonValue::String`

Two details matter here:

1. The number parser captures the **entire** matched number slice, so `-12`
   stays `-12` instead of accidentally becoming `12`.
2. The string rule is still intentionally small: it accepts any non-quote,
   non-backslash, non-control character, but it does **not** implement escape
   sequences yet.

Mini idea (illustrative only):

```text
// scalar = string | number | boolean | null
```

```rust
use marser::capture;
use marser::matcher::{multiple::many, one_or_more::one_or_more, optional::optional};
use marser::one_of::one_of;
use marser::parser::{
    Parser,
    ParserCombinator,
    token_parser::token_parser,
};
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

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> + Clone {
    let ws = Rc::new(many(one_of((' ', '\n', '\r', '\t'))));

    let null = capture!("null" => JsonValue::Null);
    let t = capture!("true" => JsonValue::Bool(true));
    let f = capture!("false" => JsonValue::Bool(false));
    let boolean = one_of((t, f));

    let number = capture!(
        bind_slice!((optional('-'), one_or_more('0'..='9')), n as &'src str)
            => JsonValue::Number(n.parse().unwrap())
    );

    let string_char = Rc::new(token_parser(
        |c: &char| *c != '"' && *c != '\\' && !c.is_control(),
        |c| *c,
    ));
    let raw_string = Rc::new(capture!((
        '"',
        many(bind!(string_char.clone(), *chars)),
        '"'
    ) => chars.into_iter().collect::<String>()));
    let string = raw_string.clone().map_output(JsonValue::String);

    let scalar = one_of((string, number, boolean, null));

    capture!((
        ws.clone(),
        bind!(scalar, value),
        ws.clone()
    ) => value)
}

let (value, errors) = json().parse_str(" -123 ").unwrap();
assert_eq!(value, JsonValue::Number(-123));
assert!(errors.is_empty());

let (value, errors) = json().parse_str(r#""hello world""#).unwrap();
assert_eq!(value, JsonValue::String("hello world".to_string()));
assert!(errors.is_empty());
```

At this point the parser can handle standalone scalar JSON values, but not
nested structures.

## 3) Grow it again: add arrays and objects

This is where recursion matters:

- arrays contain `value` elements
- object values are also `value`
- therefore the full parser must be defined with `recursive(...)`

This section also introduces **repeated binds** such as `*items` and
`*entries`. Inside `capture!`, each successful `bind!(..., *items)` appends one
more element to a `Vec<_>`.

Mini idea (illustrative only):

```text
// array  = '[' [ value (',' value)* ] ']'
// object = '{' [ string ':' value (',' string ':' value)* ] '}'
```

```rust
use marser::capture;
use marser::matcher::{multiple::many, one_or_more::one_or_more, optional::optional};
use marser::one_of::one_of;
use marser::parser::{
    deferred::recursive,
    token_parser::token_parser,
    Parser,
    ParserCombinator,
};
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

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> + Clone {
    recursive(|value| {
        let ws = Rc::new(many(one_of((' ', '\n', '\r', '\t'))));

        let null = capture!("null" => JsonValue::Null);
        let t = capture!("true" => JsonValue::Bool(true));
        let f = capture!("false" => JsonValue::Bool(false));
        let boolean = one_of((t, f));

        let number = capture!(
            bind_slice!((optional('-'), one_or_more('0'..='9')), n as &'src str)
                => JsonValue::Number(n.parse().unwrap())
        );

        let string_char = Rc::new(token_parser(
            |c: &char| *c != '"' && *c != '\\' && !c.is_control(),
            |c| *c,
        ));
        let raw_string = Rc::new(capture!((
            '"',
            many(bind!(string_char.clone(), *chars)),
            '"'
        ) => chars.into_iter().collect::<String>()));
        let string = raw_string.clone().map_output(JsonValue::String);

        let key_value = Rc::new(capture!((
            bind!(raw_string.clone(), key),
            ws.clone(),
            ':',
            ws.clone(),
            bind!(value.clone(), val)
        ) => (key, val)));

        let array = capture!((
            '[',
            ws.clone(),
            optional((
                bind!(value.clone(), *items),
                many((
                    ',',
                    ws.clone(),
                    bind!(value.clone(), *items)
                ))
            )),
            ']'
        ) => JsonValue::Array(items));

        let object = capture!((
            '{',
            ws.clone(),
            optional((
                bind!(key_value.clone(), *entries),
                many((
                    ',',
                    ws.clone(),
                    bind!(key_value.clone(), *entries)
                ))
            )),
            '}'
        ) => JsonValue::Object(entries));

        let value_core = one_of((object, array, string, number, boolean, null));

        capture!((
            ws.clone(),
            bind!(value_core, out),
            ws.clone()
        ) => out)
    })
}

let src = r#"{"ok": true, "items": [1, -2, 3], "msg": "hello"}"#;
let (value, errors) = json().parse_str(src).unwrap();

assert!(errors.is_empty());
assert_eq!(
    value,
    JsonValue::Object(vec![
        ("ok".to_string(), JsonValue::Bool(true)),
        (
            "items".to_string(),
            JsonValue::Array(vec![
                JsonValue::Number(1),
                JsonValue::Number(-2),
                JsonValue::Number(3),
            ]),
        ),
        ("msg".to_string(), JsonValue::String("hello".to_string())),
    ])
);
```

At this point, you have a complete recursive JSON parser for a small but useful
subset of JSON.

## 4) Complete parser block

The block below is the full tutorial result in one place. It is kept as a
single runnable example so it can be exercised by `cargo test`.

```rust
use marser::capture;
use marser::matcher::{multiple::many, one_or_more::one_or_more, optional::optional};
use marser::one_of::one_of;
use marser::parser::{
    deferred::recursive,
    token_parser::token_parser,
    Parser,
    ParserCombinator,
};
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

fn json<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> + Clone {
    recursive(|value| {
        let ws = Rc::new(many(one_of((' ', '\n', '\r', '\t'))));

        let null = capture!("null" => JsonValue::Null);
        let t = capture!("true" => JsonValue::Bool(true));
        let f = capture!("false" => JsonValue::Bool(false));
        let boolean = one_of((t, f));

        let number = capture!(
            bind_slice!((optional('-'), one_or_more('0'..='9')), n as &'src str)
                => JsonValue::Number(n.parse().unwrap())
        );

        let string_char = Rc::new(token_parser(
            |c: &char| *c != '"' && *c != '\\' && !c.is_control(),
            |c| *c,
        ));
        let raw_string = Rc::new(capture!((
            '"',
            many(bind!(string_char.clone(), *chars)),
            '"'
        ) => chars.into_iter().collect::<String>()));
        let string = raw_string.clone().map_output(JsonValue::String);

        let key_value = Rc::new(capture!((
            bind!(raw_string.clone(), key),
            ws.clone(),
            ':',
            ws.clone(),
            bind!(value.clone(), val)
        ) => (key, val)));

        let array = capture!((
            '[',
            ws.clone(),
            optional((
                bind!(value.clone(), *items),
                many((
                    ',',
                    ws.clone(),
                    bind!(value.clone(), *items)
                ))
            )),
            ']'
        ) => JsonValue::Array(items));

        let object = capture!((
            '{',
            ws.clone(),
            optional((
                bind!(key_value.clone(), *entries),
                many((
                    ',',
                    ws.clone(),
                    bind!(key_value.clone(), *entries)
                ))
            )),
            '}'
        ) => JsonValue::Object(entries));

        let value_core = one_of((object, array, string, number, boolean, null));

        capture!((
            ws.clone(),
            bind!(value_core, out),
            ws.clone()
        ) => out)
    })
}

let (value, errors) = json().parse_str("null").unwrap();
assert_eq!(value, JsonValue::Null);
assert!(errors.is_empty());

let (value, errors) = json().parse_str(" -42 ").unwrap();
assert_eq!(value, JsonValue::Number(-42));
assert!(errors.is_empty());

let (value, errors) = json().parse_str(r#"["a", true, null]"#).unwrap();
assert_eq!(
    value,
    JsonValue::Array(vec![
        JsonValue::String("a".to_string()),
        JsonValue::Bool(true),
        JsonValue::Null,
    ])
);
assert!(errors.is_empty());

let src = r#"{"ok": true, "items": [1, 2, 3], "name": "demo"}"#;
let (value, errors) = json().parse_str(src).unwrap();
assert_eq!(
    value,
    JsonValue::Object(vec![
        ("ok".to_string(), JsonValue::Bool(true)),
        (
            "items".to_string(),
            JsonValue::Array(vec![
                JsonValue::Number(1),
                JsonValue::Number(2),
                JsonValue::Number(3),
            ]),
        ),
        ("name".to_string(), JsonValue::String("demo".to_string())),
    ])
);
assert!(errors.is_empty());
```

## 5) What to improve next

- **Escaped strings**: support `\"`, `\\`, `\n`, and unicode escapes.
- **Full numbers**: add fractional and exponent forms such as `1.2` and `1e3`.
- **Recovery**: add `recover_with(...)` so malformed sections can still produce
  fallback nodes.
- **Diagnostics**: attach richer labels or notes with `add_error_info(...)`.
- **Commit points**: add `commit_on(...)` around arrays, objects, numbers, or
  strings when you want better error behavior in larger grammars.

If you want to compare this tutorial grammar with a more realistic one:

- `examples/json/grammar.rs` adds richer validation, recovery, and optional tracing (run via the `json` example in `examples/json/main.rs`)
- [Errors and Recovery](crate::guide::errors_and_recovery) explains when to
  commit and recover
- [Common patterns](crate::guide::common_patterns) collects the reusable recipes
  used here

After this page, continue with [Errors and Recovery](crate::guide::errors_and_recovery).
