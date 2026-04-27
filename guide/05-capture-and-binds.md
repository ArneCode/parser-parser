# Capture and Binds

Most `marser` grammars use matchers to describe input and `capture!` to turn the
matched input into Rust values.

This chapter explains:

- what `capture!` does
- when to use `bind!`, `bind_span!`, and `bind_slice!`
- how single, repeated, and optional binds behave
- how to avoid common bind-shape mistakes
- when zero-copy parsing with `bind_slice!` is useful

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

## Start small

The smallest useful capture binds one parser result and returns it:

```rust,ignore
capture!(
    bind!('x', ch) => ch
)
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

```rust,ignore
let name = capture!(
    bind_slice!(
        (
            one_of(('a'..='z', 'A'..='Z', '_')),
            many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
        ),
        text
    ) => text
);
```

The matcher recognizes the spelling of a name. `bind_slice!` returns the slice of
input that was consumed. This avoids allocating a new `String`.

### Step 2: add a span

If later diagnostics need to point at the name, capture the span too:

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

```rust,ignore
let name_list = capture!(
    (
        bind!(name.clone(), *names),
        many((',', bind!(name.clone(), *names))),
    ) => names
);
```

Each successful `bind!(name.clone(), *names)` appends one value to `names`.

### Step 4: capture optional syntax

An optional bind uses `?name` and produces an `Option<_>`:

```rust,ignore
let declaration = capture!(
    (
        bind!(name.clone(), name),
        optional((':', bind!(name.clone(), ?ty))),
    ) => Declaration { name, ty }
);
```

If the annotation is present, `ty` is `Some(...)`. If it is absent, `ty` is
`None`.

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

```rust,ignore
capture!(
    bind_span!('@', at_sign) => at_sign
)
```

Use it when diagnostics or later AST nodes need a source location, but not the
matched text.

### `bind_slice!`

`bind_slice!(matcher, text)` runs a matcher and stores the input slice covered by
that matcher:

```rust,ignore
capture!(
    bind_slice!(one_or_more('0'..='9'), digits) => digits
)
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

## Bind placement rules

`capture!` stores values in generated slots:

- a plain bind writes one required slot
- an optional bind writes one optional slot
- a repeated bind appends to a vector

That means bind shape must follow control flow.

Good repeated bind:

```rust,ignore
capture!(
    many(bind!(digit, *digits)) => digits
)
```

Bad repeated bind:

```rust,ignore
capture!(
    many(bind!(digit, digit)) => digit
)
```

The bad shape may compile, but the first iteration sets the single slot and a
later iteration tries to set the same slot again.

Good optional bind:

```rust,ignore
capture!(
    optional(bind!(sign_parser, ?sign)) => sign
)
```

Bad optional bind:

```rust,ignore
capture!(
    optional(bind!(sign_parser, sign)) => sign
)
```

The bad shape may compile, but the overall grammar can succeed without setting
the required `sign` slot.

When bind shapes are wrong, errors usually show up as runtime panics:

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

```rust,ignore
capture!(
    bind!(identifier_parser, ident, ident_span)
        => IdentNode { ident, span: ident_span }
)
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

```rust,ignore
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
```

The trade-off is lifetime coupling. The output borrows from the input, so it
cannot outlive the source text. If you need owned data, convert the slice at the
boundary where ownership is required:

```rust,ignore
capture!(
    bind_slice!(identifier_matcher, text) => text.to_owned()
)
```

## Bind form matrix

Use this as a compact reference:

```text
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

For lower-level implementation details, see the `Capture`, `ResultBinder`,
`SpanBinder`, and `SliceBinder` entries in
[Parser and Matcher Reference](crate::guide::parser_matcher_reference).
# Capture and Binds

Most `marser` grammars use matchers to describe input and `capture!` to turn the
matched input into Rust values.

This chapter focuses on that bridge:

- what `capture!` does
- when to use `bind!`, `bind_span!`, and `bind_slice!`
- how single, optional, and repeated binds behave
- how to avoid the most common bind-shape mistakes

## The role of `capture!`

`capture!` builds a parser from two pieces:

```rust,ignore
capture!(grammar => output_expression)
```

The `grammar` side is a matcher expression. It can use literals, ranges, tuple
sequences, `one_of(...)`, `many(...)`, `optional(...)`, `commit_on(...)`, and bind
forms.

The `output_expression` side is ordinary Rust. It receives the names introduced
by bind forms and returns the parser output.

For example:

```rust,ignore
capture!(
    ('[', bind!(items_parser, items), ']') => JsonValue::Array(items)
)
```

This creates a parser. When the grammar matches, `capture!` calls the generated
constructor and returns `Some(JsonValue::Array(items))`. When the grammar does
not match normally, it returns `None`. 

You usually use `capture!` when a grammar rule has a clear shape and you want to
build one output value from the interesting pieces of that shape.

## A small example

Start with a parser that recognizes an identifier and returns the original text:

```rust,ignore
let identifier = capture!(
    bind_slice!(
        (
            one_of(('a'..='z', 'A'..='Z', '_')),
            many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
        ),
        text
    ) => text
);
```

The matcher describes the valid characters. `bind_slice!` stores the consumed
source slice, so the result expression can return `text` without allocating a new
`String`.

If diagnostics also need a location, bind a span:

```rust,ignore
let identifier = capture!(
    bind_span!(
        bind_slice!(
            (
                one_of(('a'..='z', 'A'..='Z', '_')),
                many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            ),
            text
        ),
        span
    ) => Ident { text, span }
);
```

For a parser that normalizes the text instead, use `bind!` around a parser that
produces the normalized value:

```rust,ignore
capture!(
    bind!(identifier_parser, ident) => AstNode::Ident(ident)
)
```

## The three bind macros

`capture!` recognizes three bind macros inside the grammar.

### `bind!`

`bind!(parser, name)` runs a parser and stores its output.

Use it when you need the parsed meaning of a grammar part:

```rust,ignore
capture!(
    (bind!(raw_string.clone(), key), ':', bind!(value.clone(), value))
        => (key, value)
)
```

`bind!` can also capture the span consumed by that same parser run:

```rust,ignore
bind!(identifier, name, name_span)
```

The first target stores the parser output. The second target stores the consumed
`(start, end)` span.

### `bind_span!`

`bind_span!(matcher, span)` runs a matcher and stores only the consumed span.

Use it when diagnostics or later AST nodes need a source location, but not the
matched text:

```rust,ignore
capture!(
    bind_span!('"', quote_span) => quote_span
)
```

This is useful for labels, highlights, recovery notes, and error context.

### `bind_slice!`

`bind_slice!(matcher, text)` runs a matcher and stores the input slice covered by
that matcher.

Use it when exact source spelling matters:

```rust,ignore
capture!(
    bind_slice!(one_or_more('0'..='9'), digits_text) => digits_text
)
```

This is useful for identifiers, number literals, invalid fragments, comments, and
lossless syntax trees.

## Single, repeated, and optional binds

Every bind target has one of three shapes:

```rust,ignore
bind!(parser, name)        // exactly one; result sees name: T
bind!(parser, *names)      // zero or more; result sees names: Vec<T>
bind!(parser, ?name)       // zero or one; result sees name: Option<T>
```

The same shapes work with `bind_span!`, `bind_slice!`, and the span target in
`bind!(parser, value, span)`.

The shape should describe how many times that bind can run on a successful parse
path:

- Use a plain `name` when the grammar must execute the bind exactly once.
- Use `*name` when the bind appears in repeated grammar or should collect many
  occurrences.
- Use `?name` when the grammar may succeed without executing the bind.

## Bind placement rules

Bind placement is important because `capture!` stores captures in slots.

A plain single bind stores into one required slot. An optional bind stores into
one optional slot. A repeated bind stores into a vector.

That gives the main rules:

- Inside `many(...)` or other repeated grammar, use `*name`.
- Inside `optional(...)`, use `?name` unless another part of the grammar
  guarantees the bind always runs.
- Use a plain `name` only where the successful grammar path must run it exactly
  once.

Good:

```rust,ignore
capture!(
    many(bind!(digit, *digits)) => digits
)

capture!(
    optional(bind!(sign_parser, ?sign)) => sign
)
```

Bad:

```rust,ignore
capture!(
    many(bind!(digit, digit)) => digit
)

capture!(
    optional(bind!(sign_parser, sign)) => sign
)
```

These bad shapes may compile, but they do not describe the runtime behavior
correctly. A single bind inside `many(...)` may try to write the same single slot
more than once. A required bind inside `optional(...)` may be absent even though
the overall grammar succeeds.

When that happens, the failure usually appears later as a duplicate-bind panic or
as a required match result that was never set.

## Binds inside choices

`one_of(...)` tries alternatives from left to right. Be careful when different
branches bind different names.

This is usually awkward:

```rust,ignore
capture!(
    one_of((
        bind!(string_parser, string),
        bind!(number_parser, number),
    )) => JsonValue::from_parts(string, number)
)
```

Only one branch runs, so the other single bind is unset. Prefer making each branch
produce its own parser output, then choose between those parsers:

```rust,ignore
let string_value = capture!(bind!(string_parser, value) => JsonValue::String(value));
let number_value = capture!(bind!(number_parser, value) => JsonValue::Number(value));

let value = one_of((string_value, number_value));
```

If alternatives are just different spellings of the same thing, bind the same
optional or repeated shape only when the output expression can handle absence.

## Value plus span

`bind!` has a value-plus-span form:

```rust,ignore
bind!(parser, value, span)
bind!(parser, *values, *spans)
bind!(parser, ?value, ?span)
```

Use this when you need both parsed meaning and source location.

For example, an identifier parser may return a normalized symbol, while the span
points back to the original source:

```rust,ignore
capture!(
    bind!(identifier_parser, ident, ident_span)
        => IdentNode { ident, span: ident_span }
)
```

The value target and span target do not have to use the same shape, but they
usually should. If a parser runs repeatedly, both the values and spans normally
belong in vectors.

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

## Zero-copy parsing with `bind_slice!`

`bind_slice!` is the zero-copy bind form. Instead of building a new `String` or
`Vec` from matched tokens, it stores a borrowed view into the original input.

That is useful for performance:

- fewer allocations
- less copying
- exact source spelling is preserved

It is also useful for tooling. Formatters, diagnostics, and lossless syntax trees
often need the original text, not a normalized value.

For example, a number parser can keep the original literal:

```rust,ignore
capture!(
    bind_slice!(
        (
            optional('-'),
            one_or_more('0'..='9'),
            optional(('.', one_or_more('0'..='9'))),
        ),
        number_text
    ) => JsonNumber::Raw(number_text)
)
```

The trade-off is lifetime coupling. The output borrows from the input, so it
cannot outlive the source text. If you need owned data, convert the slice at the
boundary where ownership is required:

```rust,ignore
capture!(
    bind_slice!(identifier_matcher, text) => text.to_owned()
)
```

A useful rule of thumb:

- Use `bind_slice!` for exact source text.
- Use `bind!` for parsed meaning.
- Use `bind_span!` for locations.

## Quick reference

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

## Designing captures

Good capture design keeps parser rules predictable:

- Match structure with matchers and capture only the values the AST or diagnostic
  layer needs.
- Let bind shape follow control flow: repeated grammar gets `*`, optional grammar
  gets `?`, and required grammar gets a plain bind.
- Prefer branch-local `capture!` parsers for alternatives that produce different
  output shapes.
- Prefer `bind_slice!` for source text you can borrow, especially for identifiers,
  literals, invalid fragments, and lossless parsing.
- Prefer `bind_span!` when diagnostics only need a location.
- Parse into owned or normalized values only when later code benefits from that
  representation.
