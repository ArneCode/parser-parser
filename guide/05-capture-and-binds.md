`capture!`, `bind!`, and choosing bind shapes.

<div style="background-color: #fff8e1; border-left: 4px solid #f9a825; padding: 0.75em 1em; margin: 1em 0;">

**AI assistance:** This chapter was drafted with AI assistance while the library is still young. The guide is expected to improve over time as APIs and examples stabilize. If anything looks wrong or confusing, please [report it on GitHub](https://github.com/ArneCode/marser/issues/new).

</div>

# Capture and Binds

Most `marser` grammars use matchers to describe input and `capture!` to turn the
matched input into Rust values.

This chapter explains:

- what `capture!` does
- when to use `bind!`, `bind_span!`, and `bind_slice!`
- how single, repeated, and optional binds behave
- how to avoid common bind-shape mistakes
- when zero-copy parsing with `bind_slice!` is useful

If you prefer the short version first, jump to [Quick reference (bind shapes)](#quick-reference-bind-shapes) and then come back for the worked explanations.

## Before you memorize syntax

Three ideas explain most of this page:

1. `capture!` runs **matcher-shaped grammar** and then builds **parser output**.
2. Every bind target has a **shape**: exactly one (`name`), repeated (`*name`), or optional (`?name`).
3. That shape should follow **control flow**: repeated grammar needs `*`, optional grammar needs `?`, and always-run grammar can use a plain name.

If one of those three ideas feels off, most confusing bind errors become easy to diagnose.

## Mental model

`capture!` is the bridge between matcher syntax and parser output:

```text
input
  -> matcher grammar runs
  -> binders write capture slots
  -> result expression receives bound values
  -> parser returns output
```

The macro shape is:

```rust,ignore
capture!(grammar => output_expression)
```

The `grammar` side is a matcher expression. It can use literals, ranges, tuple
sequences, `one_of(...)`, `many(...)`, `optional(...)`, `commit_on(...)`, and bind
forms.

The `output_expression` side is ordinary Rust. It can use names introduced by
binds in the grammar.

## What expands behind the scenes

You usually should **think in terms of the source macro**, not the generated
Rust. But it helps to know the broad shape of what `capture!` builds:

1. It **scans the grammar** for `bind!`, `bind_span!`, `bind_slice!`, and
   `use_binds!`.
2. It groups bindings by **shape** into three buckets:
   - single (`name`)
   - repeated (`*name`)
   - optional (`?name`)
3. It assigns each binding name a **slot** inside those buckets.
4. It rewrites each bind site into an internal binder helper that writes into
   the appropriate slot while the matcher runs.
5. It builds a [`Capture`](crate::parser::capture::Capture) parser whose
   constructor receives the filled buckets and evaluates your `=>` result
   expression.

Conceptually, this:

```rust,ignore
capture!(
    (
        bind!(identifier(), name),
        optional((':', bind!(ty_parser(), ?ty))),
    ) => Declaration { name, ty }
)
```

turns into something closer to:

```rust,ignore
Capture::new(
    |single_props, multiple_props, optional_props| {
        (
            bind_result(identifier(), single_props.name_slot),
            optional((':', bind_result(ty_parser(), optional_props.ty_slot))),
        )
    },
    |single_values, _multiple_values, optional_values| {
        let name = /* read required single slot */;
        let ty = /* read optional slot */;
        Declaration { name, ty }
    },
)
```

That is not the literal emitted code, but it is the right mental model:

- **the grammar becomes a matcher tree**
- **binds become writes into typed slots**
- **the result expression becomes a constructor over those slots**

Internally, the buckets are tuple-shaped match results split into
`(single, multiple, optional)`, and snapshots of the same layout are what make
`use_binds!` work inside diagnostic factories.

Why this matters:

- repeated compatible binds can be merged into one slot
- incompatible sigils / explicit types can be rejected at macro time
- backtracking can subtract captures from the current result when a branch is
  abandoned
- `use_binds!` can read a stable snapshot of earlier captures without changing
  the parser result model

## Start small

The smallest useful capture binds one parser result and returns it:

```rust
use marser::capture;
use marser::parser::Parser;

let p = capture!(bind!('x', ch) => ch);
assert_eq!(p.parse_str("x").unwrap().0, 'x');
```

Here:

- `'x'` is a parser over `char` input.
- `bind!('x', ch)` runs that parser and stores its output in `ch`.
- `=> ch` returns that captured value.

If the input starts with `x`, this parser returns `Some('x')`. If it does not,
the parser returns `None`.

## Parser arguments vs matcher arguments

The bind macros accept different kinds of inner expressions:

- `bind!(parser, name)` needs a **parser**, because it stores parser output.
- `bind_span!(matcher, span)` needs a **matcher**, because it stores where a
  matcher succeeded.
- `bind_slice!(matcher, text)` needs a **matcher**, because it stores the source
  slice consumed by a matcher.

Use `bind!` for meaning, `bind_span!` for location, and `bind_slice!` for source
text.

## A progressive example

Suppose a small language has names, comma-separated name lists, and optional type
annotations:

```text
name
name, other, third
name: Type
```

### Step 1: capture source text

Start with a name parser that returns the original source text:

```rust
use marser::capture;
use marser::matcher::multiple::many;
use marser::one_of::one_of;
use marser::parser::Parser;

fn name_parser<'src>() -> impl Parser<'src, &'src str, Output = &'src str> + Clone {
    capture!(
        bind_slice!(
            (
                one_of(('a'..='z', 'A'..='Z', '_')),
                many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            ),
            text
        ) => text
    )
}

assert_eq!(name_parser().parse_str("foo12").unwrap().0, "foo12");
```

The matcher recognizes the spelling of a name. `bind_slice!` returns the slice of
input that was consumed. This avoids allocating a new `String`.

### Step 2: add a span

If later diagnostics need to point at the name, capture the span too. Nesting
`bind_span!` around `bind_slice!` is a common pattern (the `capture!` macro
rewrites the inner `bind_slice!` for you):

```rust,ignore
let name_node = capture!(
    bind_span!(
        bind_slice!(
            (
                one_of(('a'..='z', 'A'..='Z', '_')),
                many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            ),
            text
        ),
        span
    ) => Name { text, span }
);
```

`bind_span!` stores the consumed `(start, end)` positions. The errors chapter
uses spans to add extra diagnostic labels; see
[Errors and Recovery](crate::guide::errors_and_recovery).

### Step 3: collect repeated values

A repeated bind uses `*name` and produces a `Vec<_>`:

```rust
use marser::capture;
use marser::matcher::multiple::many;
use marser::one_of::one_of;
use marser::parser::Parser;

fn name_parser<'src>() -> impl Parser<'src, &'src str, Output = &'src str> + Clone {
    capture!(
        bind_slice!(
            (
                one_of(('a'..='z', 'A'..='Z', '_')),
                many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            ),
            text
        ) => text
    )
}

fn name_list_parser<'src>() -> impl Parser<'src, &'src str, Output = Vec<&'src str>> + Clone {
    let name = name_parser();
    capture!(
        (
            bind!(name.clone(), *names),
            many((',', bind!(name.clone(), *names))),
        ) => names
    )
}

assert_eq!(
    name_list_parser().parse_str("a,b,c").unwrap().0,
    vec!["a", "b", "c"]
);
```

Each successful `bind!(name.clone(), *names)` appends one value to `names`.

### Step 4: capture optional syntax

An optional bind uses `?name` and produces an `Option<_>`:

```rust
use marser::capture;
use marser::matcher::optional::optional;
use marser::parser::{token_parser, Parser};

fn annotated_digit<'src>() -> impl Parser<'src, &'src str, Output = (u32, Option<u32>)> + Clone {
    let digit = token_parser(
        |c: &char| c.is_ascii_digit(),
        |c| c.to_digit(10).unwrap(),
    );
    capture!((
        bind!(digit.clone(), lhs),
        optional((':', bind!(digit.clone(), ?rhs))),
    ) => (lhs, rhs))
}

assert_eq!(annotated_digit().parse_str("1:2").unwrap().0, (1, Some(2)));
assert_eq!(annotated_digit().parse_str("3").unwrap().0, (3, None));
```

The earlier steps used slices for names; here a tiny digit grammar keeps the
optional-bind idea obvious: if the `':'` branch is present, `rhs` is `Some(...)`.
If it is absent, `rhs` is `None`.

## The three bind macros

### `bind!`

`bind!(parser, name)` runs a parser and stores its output.

Use it when you need the parsed meaning of a grammar part:

```rust,ignore
capture!(
    (bind!(name.clone(), key), ':', bind!(expr.clone(), value))
        => Pair { key, value }
)
```

Use `bind!` when the inner parser already produces the value you want: an AST
node, a normalized number, an enum variant, or any other semantic value.

### `bind_span!`

`bind_span!(matcher, span)` runs a matcher and stores only the consumed span:

```rust
use marser::capture;
use marser::parser::Parser;

fn at_sign_span<'src>() -> impl Parser<'src, &'src str, Output = (usize, usize)> + Clone {
    capture!(bind_span!('@', at_sign) => at_sign)
}

assert_eq!(at_sign_span().parse_str("@").unwrap().0, (0, 1));
```

Use it when diagnostics or later AST nodes need a source location, but not the
matched text.

### `bind_slice!`

`bind_slice!(matcher, text)` runs a matcher and stores the input slice covered by
that matcher:

```rust
use marser::capture;
use marser::matcher::one_or_more::one_or_more;
use marser::parser::Parser;

fn digits_slice<'src>() -> impl Parser<'src, &'src str, Output = &'src str> + Clone {
    capture!(bind_slice!(one_or_more('0'..='9'), digits) => digits)
}

assert_eq!(digits_slice().parse_str("4096").unwrap().0, "4096");
```

Use it when exact source spelling matters: identifiers, number literals, invalid
fragments, comments, and lossless syntax trees.

`bind_slice!` captures raw text, not semantic meaning. For example, a string
literal slice still contains escape syntax, and a number slice still needs to be
validated or converted if later code needs a number.

## Bind shapes

Every bind target has one of three shapes:

```rust,ignore
bind!(parser, name)        // exactly one; result sees name: T
bind!(parser, *names)      // zero or more; result sees names: Vec<T>
bind!(parser, ?name)       // zero or one; result sees name: Option<T>
```

The same shapes work with `bind_span!`, `bind_slice!`, and the span target in
`bind!(parser, value, span)`.

Choose the shape based on how many times that bind can run on a successful parse
path:

- Use plain `name` when the grammar must execute the bind exactly once.
- Use `*name` when the bind appears in repeated grammar or should collect many
  occurrences.
- Use `?name` when the grammar may succeed without executing the bind.

## A fast decision rule

When you are choosing a bind form, ask:

- Do I want the **semantic output** of a parser? Use `bind!`.
- Do I only want the **location** of syntax? Use `bind_span!`.
- Do I want the **exact source text** that matched? Use `bind_slice!`.

Then ask how many times that bind can run on a successful path:

- exactly once -> `name`
- zero or more times -> `*name`
- zero or one time -> `?name`

## Bind placement rules

`capture!` stores values in generated slots:

- a plain bind writes one required slot
- an optional bind writes one optional slot
- a repeated bind appends to a vector

That means bind shape must follow control flow.

`capture!` is already defensive about some mistakes. Several invalid forms are rejected at **macro expansion time**, not later at runtime. The compile-fail tests under `tests/ui/` cover examples such as:

- mixing incompatible sigils for the same binding name (`x` vs `*x` vs `?x`)
- giving the same binding conflicting explicit `as` types
- using the same identifier for both the value and span targets in `bind!(..., value, span)`
- passing extra trailing arguments to `bind!`

The remaining mistakes to watch for are the ones where the syntax is valid but the chosen bind **shape** does not match the grammar path.

Good repeated bind:

```rust
use marser::capture;
use marser::matcher::multiple::many;
use marser::parser::{token_parser, Parser};

fn many_digits<'src>() -> impl Parser<'src, &'src str, Output = Vec<u32>> + Clone {
    let digit = token_parser(
        |c: &char| c.is_ascii_digit(),
        |c| c.to_digit(10).unwrap(),
    );
    capture!(many(bind!(digit, *digits)) => digits)
}

let _ = many_digits();
```

Bad repeated bind (shape mismatch: `many` can run the bind multiple times, but
the target is a single value, not `*digits`):

```rust,ignore
capture!(
    many(bind!(digit, digit)) => digit
)
```

This kind of shape mismatch is logically wrong because the grammar can execute
the bind many times while the target only has room for one value.

Good optional bind:

```rust
use marser::capture;
use marser::matcher::optional::optional;
use marser::parser::{token_parser, Parser};

fn optional_sign<'src>() -> impl Parser<'src, &'src str, Output = Option<char>> + Clone {
    let sign_parser = token_parser(
        |c: &char| *c == '+' || *c == '-',
        |c| *c,
    );
    capture!(optional(bind!(sign_parser, ?sign)) => sign)
}

assert_eq!(optional_sign().parse_str("-").unwrap().0, Some('-'));
assert_eq!(optional_sign().parse_str("").unwrap().0, None);
```

Bad optional bind (shape mismatch: `optional` may skip the bind, but `sign` is
not optional):

```rust,ignore
capture!(
    optional(bind!(sign_parser, sign)) => sign
)
```

This kind of shape mismatch is logically wrong because the grammar can succeed
without ever assigning the required `sign` slot.

When a shape mismatch is not rejected at macro time, the failure usually shows
up later during parsing or output construction:

- a single or optional slot was set more than once
- a required single slot was never set before output construction

## Binds inside choices

`one_of(...)` tries alternatives from left to right. Be careful when alternatives
bind different required names.

This is usually wrong:

```rust,ignore
capture!(
    one_of((
        bind!(string_parser, string),
        bind!(number_parser, number),
    )) => Value::from_parts(string, number)
)
```

Only one branch runs, so the other required bind is unset.

As a rule of thumb: if branches mean **different semantic cases**, make each
branch build its own output and then choose between those parsers. Do not try to
share several unrelated required bind names across one outer `capture!`.

Prefer making each branch produce its own parser output, then choose between
those parsers:

```rust,ignore
let string_value = capture!(bind!(string_parser, value) => Value::String(value));
let number_value = capture!(bind!(number_parser, value) => Value::Number(value));

let value = one_of((string_value, number_value));
```

If alternatives are different spellings of the same concept, bind the same name
and shape from each branch only when that shape is valid for the result.

## Value plus span

`bind!` can capture parser output and the span consumed by that parser:

```rust,ignore
bind!(parser, value, span)
bind!(parser, *values, *spans)
bind!(parser, ?value, ?span)
```

Use this when you need both parsed meaning and source location:

```rust
use marser::capture;
use marser::parser::{token_parser, Parser};

#[derive(Debug, PartialEq)]
struct IdentNode {
    ident: char,
    span: (usize, usize),
}

fn ident_with_span<'src>() -> impl Parser<'src, &'src str, Output = IdentNode> + Clone {
    let identifier_parser = token_parser(
        |c: &char| c.is_ascii_lowercase(),
        |c| *c,
    );
    capture!(
        bind!(identifier_parser, ident, ident_span) => IdentNode {
            ident,
            span: ident_span,
        }
    )
}

assert_eq!(
    ident_with_span().parse_str("x").unwrap().0,
    IdentNode {
        ident: 'x',
        span: (0, 1),
    }
);
```

Keep the value and span shapes the same unless you have a specific reason not to.
If the parser can run many times, both values and spans usually belong in
vectors. If the parser is optional, both usually belong in `Option`.

## Typed bind targets

Bind targets can include an explicit type when inference needs help:

```rust,ignore
bind!(digit, *digits as char)
bind!(maybe_sign, ?sign as char)
bind_slice!(number_matcher, text as &'src str)
```

The sigil still controls the outer shape:

- `name as T` gives `T`
- `*name as T` gives `Vec<T>`
- `?name as T` gives `Option<T>`

Use explicit types sparingly. They are most useful when Rust cannot infer a
closure output, slice type, or repeated capture type.

## `use_binds!` in diagnostic factories

Most of this chapter is about building the **result** of a parser, but the same
captured values can also help build **diagnostics**.

`use_binds!(|ctx| { ... })` is meant for inline-error factories such as
`err_if_no_match(...)` and `err_if_matched(...)`. It gives the factory access to
the binds that were already captured earlier in the same `capture!`, plus a
diagnostic context value.

That is useful when an error message should point back to syntax you already
matched. For example, after reading an opening parenthesis you may want a
"missing closing parenthesis" error that also highlights where the opening `(`
appeared.

Shape sketch:

```rust,ignore
capture!(
    (
        bind_span!('(', open_paren_span),
        /* ... more grammar ... */
        ')'.err_if_no_match(use_binds!(|ctx| {
            InlineError::new("missing closing parenthesis")
                .with_span(Some(ctx.span()))
                .with_annotation(
                    open_paren_span.copied().unwrap(),
                    "opened here",
                    AnnotationKind::Context,
                )
        }))
    ) => ...
)
```

Things to remember:

- `use_binds!` is for **diagnostic builders**, not normal parser output.
- It only makes sense **inside the grammar** of `capture!` (not in the `=>` result expression), where bind snapshots exist.
- The macro expands each site to a `__UseBindsSite::<N>` type that implements [`BuildInlineError`](crate::error::BuildInlineError) with the same `'snap` snapshot model as [`SnapshotFactory`](crate::error::SnapshotFactory) (hand-written factories can use `SnapshotFactory` directly).
- It reads the captures that were already established on the successful path up
  to that point in the grammar.
- The `ctx` argument gives you the current diagnostic span / insertion point,
  while the captured names let you refer back to earlier syntax.

If you want the full diagnostic story, continue with
[Errors and Recovery](crate::guide::errors_and_recovery).

## Zero-copy parsing with `bind_slice!`

`bind_slice!` is the zero-copy bind form. Instead of building a new `String` or
`Vec` from matched tokens, it stores a borrowed view into the original input.

That is useful for performance:

- fewer allocations
- less copying
- exact source spelling is preserved

It is also useful for tooling. Formatters, diagnostics, and lossless syntax trees
often need original text, not normalized values.

Example:

```rust
use marser::capture;
use marser::matcher::{multiple::many, one_or_more::one_or_more, optional::optional};
use marser::parser::Parser;

enum NumberLiteral<'a> {
    Raw(&'a str),
}

fn raw_number<'src>() -> impl Parser<'src, &'src str, Output = NumberLiteral<'src>> + Clone {
    capture!(
        bind_slice!(
            (
                optional('-'),
                one_or_more('0'..='9'),
                optional(('.', one_or_more('0'..='9'))),
            ),
            number_text
        ) => NumberLiteral::Raw(number_text)
    )
}

assert!(matches!(
    raw_number().parse_str("-12.34").unwrap().0,
    NumberLiteral::Raw("-12.34")
));
```

The trade-off is lifetime coupling. The output borrows from the input, so it
cannot outlive the source text. If you need owned data, convert the slice at the
boundary where ownership is required:

```rust
use marser::capture;
use marser::matcher::multiple::many;
use marser::one_of::one_of;
use marser::parser::Parser;

fn ident_to_string<'src>() -> impl Parser<'src, &'src str, Output = String> + Clone {
    let identifier_matcher = many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_')));
    capture!(bind_slice!(identifier_matcher, text) => String::from(text))
}

assert_eq!(ident_to_string().parse_str("abc").unwrap().0, "abc");
```

## Bind form matrix

Use this as a compact reference:

```rust,ignore
bind!(parser, value)
  captures: parser output
  result:   value: T
  use when: exactly one semantic value is required

bind!(parser, *values)
  captures: parser outputs
  result:   values: Vec<T>
  use when: repeated grammar collects many values

bind!(parser, ?value)
  captures: parser output if present
  result:   value: Option<T>
  use when: optional grammar may not run

bind!(parser, value, span)
  captures: parser output and source span
  result:   value: T, span: (Pos, Pos)
  use when: semantic value also needs a diagnostic/source location

bind_span!(matcher, span)
  captures: source span
  result:   span: (Pos, Pos)
  use when: only the location matters

bind_slice!(matcher, text)
  captures: source slice
  result:   text: Inp::Slice
  use when: exact source text should be borrowed
```

Add `*` or `?` to `bind_span!` and `bind_slice!` targets the same way as
`bind!`: `*spans` becomes `Vec<(Pos, Pos)>`, and `?text` becomes
`Option<Inp::Slice>`.

## Common mistakes

### Plain bind inside repetition

Use `*items`, not `item`, inside `many(...)`.

### Plain bind inside optional grammar

Use `?item`, not `item`, inside `optional(...)` unless another part of the
grammar guarantees the bind runs.

### Different required binds in `one_of(...)`

If only one branch runs, required binds from the other branches are unset. Prefer
branch-local `capture!` parsers that each produce the same output type.

### Assuming every bind mistake becomes a runtime bug

Some mistakes are caught earlier. `capture!` already rejects a number of invalid
bind forms during macro expansion, and the `trybuild` tests in `tests/ui/`
exercise examples such as incompatible sigils, conflicting explicit types,
duplicate value/span names, and trailing `bind!` arguments.

### Using `bind_slice!` when owned data is needed

`bind_slice!` borrows from input. If the parsed value must outlive the source,
convert to an owned value at the boundary.

### Treating raw slices as normalized values

A slice preserves spelling. It does not decode escapes, validate a number, or
intern an identifier by itself.

### Forgetting `*` always means a vector

`*items` is `Vec<T>` even if it matched exactly once.

## Designing captures

Good capture design keeps parser rules predictable:

- Match structure with matchers and capture only what the AST or diagnostic layer
  needs.
- Let bind shape follow control flow: repeated grammar gets `*`, optional grammar
  gets `?`, and required grammar gets a plain bind.
- Prefer branch-local `capture!` parsers for alternatives that produce different
  output shapes.
- Prefer `bind_slice!` for source text you can borrow, especially identifiers,
  literals, invalid fragments, and lossless parsing.
- Prefer `bind_span!` when diagnostics only need a location.
- Parse into owned or normalized values only when later code benefits from that
  representation.

A useful workflow when designing a new `capture!`:

1. Write the matcher grammar first.
2. Mark each interesting piece as **one**, **many**, or **optional**.
3. Pick `bind!`, `bind_span!`, or `bind_slice!` based on whether you need meaning, location, or source text.
4. Only then write the output expression.

That order tends to prevent most bind-shape mistakes before the compiler or
tests need to point them out.

For lower-level implementation details, see the `Capture`, `ResultBinder`,
`SpanBinder`, and `SliceBinder` entries in
[Parser and Matcher Reference](crate::guide::parser_matcher_reference).

## Quick reference (bind shapes)

```rust,ignore
bind!(parser, value)                    // value: T
bind!(parser, *values)                  // values: Vec<T>
bind!(parser, ?value)                   // value: Option<T>

bind!(parser, value, span)              // value: T, span: (Pos, Pos)
bind!(parser, *values, *spans)          // values: Vec<T>, spans: Vec<(Pos, Pos)>
bind!(parser, ?value, ?span)            // value: Option<T>, span: Option<(Pos, Pos)>

bind_span!(matcher, span)               // span: (Pos, Pos)
bind_span!(matcher, *spans)             // spans: Vec<(Pos, Pos)>
bind_span!(matcher, ?span)              // span: Option<(Pos, Pos)>

bind_slice!(matcher, text)              // text: Inp::Slice
bind_slice!(matcher, *texts)            // texts: Vec<Inp::Slice>
bind_slice!(matcher, ?text)             // text: Option<Inp::Slice>
```
