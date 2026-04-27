# Parser and Matcher Reference

This chapter is a compact map of the parser and matcher building blocks in
`marser`.

If you are still learning how `capture!`, `bind!`, `bind_span!`, and
`bind_slice!` work, read [Capture and Binds](crate::guide::capture_and_binds)
first. This page is meant as a reference once you know the basic grammar-writing
model.

## Parser vs matcher

`marser` separates parsing into two layers:

- A **matcher** checks that an input shape is present. It may consume input, look
  ahead, record diagnostics, or bind values into a `capture!`.
- A **parser** produces a Rust value. Parsers can be primitive token parsers,
  `capture!` parsers, choices, recursive parsers, or wrappers around other
  parsers.

In day-to-day grammar code:

- Use matchers to describe the syntax shape.
- Use parsers when a rule should produce a typed value.
- Use `capture!` to run matcher logic and build parser output.

## Core parser traits

### `Parser`

`Parser` is implemented by values that can parse input and produce an `Output`.
You normally do not implement it yourself; compose the parser types in this crate
instead.

A parser has three important outcomes:

- `Some(output)` means it matched and produced a value.
- `None` means it did not match at the current input position.
- `Err(FurthestFailError)` means a committed parse failed and should be reported.

### `ParserCombinator`

Every parser gets these extension methods:

- `memoized()` caches parse results at each input position.
- `recover_with(recovery_parser)` turns a hard failure into fallback output when
  recovery succeeds.
- `add_error_info(error_parser)` enriches hard errors with notes, help, or extra
  labels.
- `ignore_result()` runs the parser as a matcher and discards the output.
- `map_output(f)` maps successful output to another value.
- `erase_types()` boxes a parser behind a stable erased type.
- `maybe_erase_types()` erases only when the `parser-erased` feature is enabled.

## Parser building blocks

### `Capture`

`Capture` is the parser behind `capture!`. It runs a matcher, collects bound
values, spans, or slices, and calls a constructor to produce output.

Use `capture!` in normal code:

```rust,ignore
capture!(
    ('"', bind_slice!(many(non_quote_char), text), '"') => text
)
```

See [Capture and Binds](crate::guide::capture_and_binds) for the full guide.

### `TokenParser` and `token_parser`

`token_parser(check_fn, parse_fn)` reads one token, checks it with `check_fn`, and
maps it with `parse_fn`. If the check fails, it rewinds and returns `None`.

```rust,ignore
let digit = token_parser(
    |c: &char| c.is_ascii_digit(),
    |c| c.to_digit(10).unwrap(),
);
```

Use this when one token should produce a transformed value.

### `SingleTokenParser`

`SingleTokenParser::new(token)` parses exactly one token equal to `token` and
returns that token. Character literals also implement `Parser` for `char` input,
so `'{'` can be used directly as a parser.

Use it for exact one-token parser rules.

### `RangeParser`, `Range`, and `RangeInclusive`

`RangeParser::new(range)` parses one token contained in a range. Rust ranges such
as `'0'..'9'` and `'0'..='9'` can also be parsers for compatible token types.

Use ranges for compact token classes such as digits or ASCII letters.

### `MultipleParser`

`MultipleParser::new(parser, combine_fn)` repeatedly runs a parser until it no
longer matches, collects the outputs into a `Vec`, and maps that vector with
`combine_fn`.

Use it for parser-level repetition where the repeated parser output should be
combined outside the matcher layer. Inside `capture!`, matcher-level `many(...)`
is usually more natural.

### `OutputMapper`

`parser.map_output(f)` preserves the matching behavior of the parser and maps
only successful output.

Use it when the parse shape is already right but the output needs a small
conversion.

### `ErrorRecoverer`

`parser.recover_with(recovery_parser)` handles hard failures by rewinding to the
start position and trying the recovery parser. If recovery succeeds, `marser`
records the original error as a collected diagnostic and returns the recovery
output.

Use it when malformed input can be represented explicitly, such as
`Invalid(slice)` or `ErrorNode`.

### `Memoized`

`parser.memoized()` caches success or absence for a parser at an input position.
Successful memoized outputs are returned as `Rc<Output>`.

Use it for expensive or recursive rules that may be reached repeatedly from the
same position.

### `Deferred`, `DeferredWeak`, and `recursive`

`recursive(...)` creates a parser that can refer to itself while it is being
built.

```rust,ignore
let value = recursive(|value| {
    one_of((object(value.clone()), array(value), string, number))
});
```

Use it for nested languages such as JSON values, parenthesized expressions, and
blocks.

### `Erased`

`parser.erase_types()` stores the parser behind a boxed trait object. This can
make large combinator types easier to name at the cost of dynamic dispatch.

`parser.maybe_erase_types()` lets the `parser-erased` Cargo feature decide
whether erasure happens.

Use it when parser types become too large or unwieldy.

### `OneOf`

`one_of((a, b, c))` is ordered choice. As a parser, every branch must produce the
same output type. Alternatives are tried left to right.

```rust,ignore
one_of((object, array, string, number, boolean, null))
```

Use it for grammar alternatives.

### `Labeled`

`parser.with_label("value")` attaches a display label to a parser. When the
parser fails softly, the label can appear in expected-token diagnostics.

Use labels at user-facing grammar boundaries, such as `"object"`, `"array"`, or
`"string literal"`.

## Core matcher traits

### `Matcher`

`Matcher` is implemented by values that can match input inside a `Capture`. You
normally compose existing matchers rather than implementing it yourself.

A matcher returns `true` for success and `false` for ordinary absence. It may
also return `Err(FurthestFailError)` when a committed match fails.

### `MatcherCombinator`

Every matcher gets these extension methods:

- `add_error_info(error_parser)` enriches hard errors from the matcher.
- `try_insert_if_missing(message)` records a synthetic missing-token error when
  the matcher fails during real error collection.

## Matcher building blocks

### Tuple sequences

A tuple such as `('(', value, ')')` is a sequential matcher. Each element must
match in order.

Use tuples for ordinary grammar sequencing. Tuples are supported up to the arity
implemented by the crate.

### `()`

The unit value is an empty matcher. It always succeeds and consumes nothing.

Use it as a no-op, or as the first part of `commit_on((), matcher)` when you want
to commit immediately.

### `AnyToken`

`AnyToken` consumes one token and succeeds if input remains. It fails at the end
of input.

Use it for catch-all recovery, unknown tokens, or end-of-input checks with
`negative_lookahead(AnyToken)`.

### `StringMatcher`, `&str`, and `char`

`StringMatcher::new(text)` matches a fixed run of `char` tokens. String slices
and character literals also implement `Matcher`, so `"true"` and `'{'` can often
be used directly.

Use these for literal syntax.

### `Range` and `RangeInclusive`

Rust ranges also implement `Matcher` for compatible token streams. They consume
one token when it is inside the range.

```rust,ignore
one_or_more('0'..='9')
```

Use ranges for character or token classes.

### `many`

`many(matcher)` is greedy zero-or-more repetition. It always succeeds, stops when
the inner matcher fails, and also stops if the inner matcher succeeds without
making progress.

Use it for whitespace, comma tails, repeated digits, and similar syntax.

When binding inside `many(...)`, use a repeated bind such as `bind!(parser,
*items)`.

### `one_or_more`

`one_or_more(matcher)` requires at least one successful match, then behaves like
greedy repetition.

Use it when at least one item is required.

### `optional`

`optional(matcher)` tries the inner matcher once and always succeeds.

Use it for syntax that may or may not be present. When binding inside
`optional(...)`, use an optional bind such as `bind!(parser, ?item)`.

### `positive_lookahead`

`positive_lookahead(matcher)` checks that the inner matcher would match at the
current position, then restores the input position.

Use it to make a decision without consuming input.

### `negative_lookahead`

`negative_lookahead(matcher)` succeeds when the inner matcher does not match at
the current position, then restores the input position.

Use it to reject trailing input, stop before a delimiter, or express "not
followed by" constraints.

### `ParserMatcher`

`ParserMatcher::new(parser, expected_output)` runs a parser as a matcher and
succeeds only when the parser output equals `expected_output`.

Use it when a parser already recognizes the syntax but a matcher needs to check a
specific parsed value.

### `IgnoreResult`

`parser.ignore_result()` runs a parser as a matcher and succeeds when the parser
returns any output. The output is discarded.

Use it when parser recognition behavior is useful inside a matcher but the output
does not need to be bound.

### `commit_on`

`commit_on(prefix, rest)` first matches `prefix`. If `prefix` succeeds, failure
inside `rest` becomes a hard error instead of ordinary absence.

Use it after the grammar has seen enough input to know which rule the user meant.
For example, after seeing `{`, an object parser should report errors inside the
object rather than silently trying another value parser.

### `ErrorContextualizer`

`matcher.add_error_info(error_parser)` wraps a matcher so hard failures can be
enriched. The `error_parser` returns a function that mutates the
`FurthestFailError`.

Use it for extra notes, help, or labels that require local context.

The same wrapper is available for parsers through `ParserCombinator`.

### `InsertOnErrorMatcher`

`matcher.try_insert_if_missing(message)` wraps a matcher so that, during real
error collection, a soft failure can be treated as an inserted missing element
and recorded as a `MissingError`.

Use it for diagnostics like "missing closing bracket" where continuing the match
produces better follow-up errors.

### `IfErrorMatcher`

`if_error(matcher)` and `if_error_else_fail(matcher)` only run the inner matcher
when a real error handler is active. Outside error collection they return a fixed
success or failure result.

Use these for grammar pieces that are meaningful only while building diagnostics.

### `UnwantedMatcher`

`unwanted(matcher, message)` succeeds when the inner matcher succeeds, but records
an `UnwantedError` for the consumed span.

Use it to recognize and report explicitly forbidden syntax while still allowing
recovery to continue.

### `OneOf`

`one_of((a, b, c))` also works as a matcher. It tries alternatives from left to
right and succeeds on the first matching branch.

Use it for alternatives inside `capture!`, such as multiple literal keywords or
valid element forms.

### `Labeled`

`matcher.with_label("label")` attaches a display label to a matcher. The label
can appear in expected-token diagnostics.

Use labels when a grammar name is clearer than a raw literal or range.

## Parser repetition vs matcher repetition

Use matcher repetition when you are still describing syntax inside `capture!`:

```rust,ignore
many((' ', '\n', '\t'))
```

Use parser repetition when each repeated parse should produce a value that is
combined outside the matcher layer:

```rust,ignore
MultipleParser::new(digit, |digits| digits.into_iter().collect::<String>())
```

The difference is where values live: matcher repetition binds through capture
properties, while parser repetition returns parser output directly.
