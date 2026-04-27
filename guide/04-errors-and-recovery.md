# Errors and Recovery

`marser` separates recognition from reporting. Most matchers can fail softly so
another grammar branch can be tried. Once the parser has enough evidence that a
specific rule is intended, you can commit to that rule and turn later failures
into user-facing diagnostics.

This chapter explains how errors occur, how they can be enriched, and how parsing
can recover and continue.

## Parse outcomes

When you call `marser::parse(parser, src)`, there are two top-level outcomes:

- `Err(FurthestFailError)` means parsing failed hard and no complete output was
  produced.
- `Ok((output, collected_errors))` means parsing produced output. The
  `collected_errors` vector may still contain recovered syntax errors.

So a successful parse can still report errors. This is important for editors,
formatters, and other tools that need an AST even when the input is imperfect.

## Soft failure vs hard failure

Most parser and matcher misses are **soft failures**:

- a parser returns `None`
- a matcher returns `false`
- `one_of(...)` can try the next alternative
- a larger grammar can decide that this branch was not the right branch

Soft failure is normal control flow. For example, a JSON value parser may try
object, array, string, number, boolean, and null. If the object parser does not
see `{`, it should fail softly so the next value kind can be tried.

A **hard failure** is an error:

- parsing returns `Err(FurthestFailError)`
- no sibling alternative should silently replace the intended rule
- diagnostics should explain what was expected at the furthest useful position

Hard failures usually come from `commit_on(...)`.

## Committing with `commit_on`

`commit_on(prefix, rest)` first matches `prefix`. If `prefix` does not match, the
whole matcher fails softly. If `prefix` does match, `marser` treats failure in
`rest` as a real syntax error.

This is the key tool for choosing when a rule becomes responsible for reporting
errors.

For a parenthesized expression, seeing the opening parenthesis is enough to
commit:

```rust,ignore
capture!(
    commit_on(
        '(',
        (
            ws.clone(),
            bind!(expr.clone(), inner),
            ws.clone(),
            ')',
        )
    ) => Expr::Group(Box::new(inner))
)
```

If there is no opening parenthesis, this is not a grouped expression and another
expression parser may try. If there is an opening parenthesis but the expression
or closing parenthesis is missing, the grouped-expression parser should report an
error.

For a function call, seeing the callee and opening `(` might be enough to commit:

```rust,ignore
capture!(
    commit_on(
        (bind!(identifier, callee), ws.clone(), '('),
        (
            ws.clone(),
            optional(bind!(argument_list, ?args)),
            ws.clone(),
            ')',
        )
    ) => Expr::Call { callee, args: args.unwrap_or_default() }
)
```

Commit points should usually be placed after a distinctive prefix: `(` for a
grouped expression, a keyword like `let`, or a delimiter that clearly starts a
specific construct.

When `rest` fails after the prefix matched, `commit_on` runs `rest` again with a
real error handler. That second pass collects expected labels and produces a
`FurthestFailError` if the committed rule still cannot match.

Later sections show how diagnostic-only matchers such as `if_error(...)`,
`try_insert_if_missing(...)`, and `unwanted(...)` can participate in that second
pass.

## Expected labels and furthest failures

When committed parsing fails, `marser` reruns the committed part with a real error
handler to collect the most useful failure. The error handler keeps the furthest
failure span and the labels that were expected there.

Labels come from things like literals, ranges, and `with_label(...)`:

```rust,ignore
let object = object_parser.with_label("object");
let string = raw_string.map_output(JsonValue::String).with_label("string");
```

Good labels make errors read like grammar concepts instead of implementation
details. Prefer `"object"`, `"array"`, or `"string literal"` over labels that
only mirror a low-level token.

The main hard error type is `FurthestFailError`. It contains:

- the primary failure span
- one or more expected labels
- optional notes
- optional help text
- optional extra labeled source spans

## Adding context with `add_error_info`

`add_error_info(error_parser)` enriches a hard `FurthestFailError`.

The `error_parser` runs near the original start position and returns a callback:

```rust,ignore
Box<dyn Fn(&mut FurthestFailError)>
```

That callback can add notes, help, or extra labels.

For example, a number parser can add a specific note for leading zeros:

```rust,ignore
number.add_error_info(one_of((
    capture!(
        (
            optional('-'),
            bind_span!('0', zero),
            '0'..='9'
        )
        => Box::new(move |err: &mut FurthestFailError| {
            err.add_extra_label(zero, "leading zero", ariadne::Color::Blue);
            err.add_note("Leading zeros are not allowed in JSON numbers");
        }) as Box<dyn Fn(&mut FurthestFailError)>
    ),
    capture!(
        (
            optional('-'),
            bind_span!('.', dot),
        )
        => Box::new(move |err: &mut FurthestFailError| {
            err.add_extra_label(dot, "missing integer part", ariadne::Color::Blue);
            err.add_note("Floating point numbers need an integer part");
        }) as Box<dyn Fn(&mut FurthestFailError)>
    ),
)))
```

Use `add_error_info(...)` when the generic "expected ..." message is correct but
could be more helpful with local explanation.

Useful mutations include:

```rust,ignore
err.add_note("Leading zeros are not allowed in JSON numbers");
err.add_help("Remove the extra zero or quote the value as a string");
err.add_extra_label(span, "leading zero", ariadne::Color::Blue);
```

## Reporting missing syntax

`matcher.try_insert_if_missing(message)` turns a soft miss into a synthetic
`MissingError`, but only during real error collection.

In normal recognition mode, it behaves like the inner matcher: if the matcher
fails, the wrapper fails too. During committed error collection, a miss records a
missing-syntax diagnostic and returns success, as if the missing piece had been
inserted.

This lets the parser continue through the rest of the rule and find more useful
follow-up errors.

Examples:

```rust,ignore
')'.try_insert_if_missing("missing closing ')'")
';'.try_insert_if_missing("missing semicolon")
','.try_insert_if_missing("missing comma")
```

Use it for delimiters and separators that are obvious from context:

- closing delimiters
- commas between list items
- semicolons after statements
- colons in field declarations

Avoid using it where insertion would be speculative. If several different tokens
could be correct, an expected-label error is usually clearer.

## Error-only grammar with `if_error`

Some grammar pieces are only useful after an error has already been detected.
Running them during normal recognition could make a grammar too permissive or too
expensive.

`if_error(matcher)` runs the inner matcher only when a real error handler is
active. In practice, that often means:

1. `commit_on(prefix, rest)` matched `prefix`
2. the first attempt to match `rest` failed
3. `commit_on` reran `rest` with a real error handler to collect diagnostics
4. `if_error(...)` saw that real error handler and ran its inner matcher

Outside that error-collection pass, `if_error(...)` succeeds without running the
inner matcher. This makes it useful for optional diagnostic cleanup that should
not affect the happy path.

Example: a list parser may want to report extra commas only after the list has
already failed normally:

```rust,ignore
capture!(
    commit_on(
        '[',
        (
            optional(bind!(item.clone(), *items)),
            many((',', bind!(item.clone(), *items))),
            if_error(many(unwanted(',', "extra comma"))),
            ']'.try_insert_if_missing("missing closing ']'"),
        )
    ) => items
)
```

On valid input, the `if_error(...)` branch is skipped. If the committed list body
fails, `commit_on` reruns the body in error mode and the extra-comma matcher can
consume and report commas that are only interesting for diagnostics.

`if_error_else_fail(matcher)` also runs only during real error collection, but
outside error collection it fails. Use it for fallback branches that should exist
only while recovering from an error.

For example, a command parser might accept unknown text as an invalid command only
after a committed command parse has already failed:

```rust,ignore
let invalid_command = capture!(
    if_error_else_fail(bind_slice!(
        one_or_more((negative_lookahead(';'), AnyToken)),
        text
    )) => Command::Invalid(text)
);
```

## Reporting unwanted syntax

`unwanted(matcher, message)` recognizes syntax that should not be present. When
the inner matcher succeeds, it records an `UnwantedError` for the consumed span
and still returns success.

This is useful when the parser can continue after consuming the unwanted input.

For example, a list parser can report trailing commas:

```rust,ignore
capture!(
    commit_on(
        '[',
        (
            optional((
                bind!(item.clone(), *items),
                many((',', bind!(item.clone(), *items))),
                if_error(optional(unwanted(',', "trailing comma"))),
            )),
            ']'.try_insert_if_missing("missing closing ']'"),
        )
    ) => items
)
```

Use `unwanted(...)` when you want to say "this was found, but it should not be
here" rather than "something was missing". It pairs well with `if_error(...)`
because unwanted syntax is often something you only want to scan for after a
normal committed parse has failed.

## Recovering with `recover_with`

`parser.recover_with(recovery_parser)` catches hard failures from `parser`.

When the happy parser returns `Err(FurthestFailError)`, `recover_with(...)`:

1. rewinds to the parser's start position
2. runs the recovery parser
3. if recovery succeeds, stores the original hard error in `collected_errors`
4. returns the recovery output

The recovery parser must produce the same output type as the happy parser.

For example, an assignment parser can recover malformed right-hand sides into an
explicit invalid expression:

```rust,ignore
assignment.recover_with(
    capture!(
        bind_slice!(
            many((negative_lookahead(';'), AnyToken)),
            slice
        ) => Stmt::Invalid(slice)
    )
)
```

This lets parsing continue after reporting the original assignment error.
Downstream code can still inspect the AST and see exactly where the invalid node
is.

Recovery only handles hard failures. If a parser returns `None`, recovery is not
needed because the parser simply did not match.

## Missing, unwanted, and furthest-fail errors

`marser` has several user-facing error shapes:

- `FurthestFailError` reports a hard syntax failure with expected labels.
- `MissingError` reports syntax that was inserted during recovery, such as a
  missing closing delimiter.
- `UnwantedError` reports syntax that was consumed but should not have appeared.

`FurthestFailError` can be returned as the top-level `Err(...)`, or it can be
collected when recovery succeeds. `MissingError` and `UnwantedError` are pushed
into the parser context as collected errors.

That is why `Ok((output, collected_errors))` can still contain syntax errors.

## Choosing the right tool

Use `commit_on(prefix, rest)` when:

- the prefix identifies the grammar rule
- later failure should be reported as an error
- trying sibling alternatives would hide the real problem

Use `with_label(...)` when:

- the expected item should be described as a grammar concept
- error output would otherwise mention only a low-level token

Use `add_error_info(...)` when:

- the generic expected-token message is correct but incomplete
- local input can explain why the syntax is invalid
- you want notes, help, or extra labels

Use `try_insert_if_missing(...)` when:

- a specific missing token is obvious from context
- pretending it was present lets parsing continue usefully
- you want a `MissingError` in the collected errors

Use `unwanted(...)` when:

- invalid syntax can be consumed safely
- you want a specific `UnwantedError`
- continuing after the unwanted syntax gives better output

Use `recover_with(...)` when:

- the happy parser can fail hard
- there is a safe fallback parser
- the fallback output can represent the invalid syntax explicitly

## Practical guidance

- Commit only after a distinctive prefix. Committing too early makes alternatives
  unavailable and can produce surprising errors.
- Add labels at grammar boundaries, not everywhere.
- Prefer recovery outputs that are explicit in the AST, such as `Invalid` or
  `ErrorNode`.
- Keep recovery local. One clear recovery strategy per rule is easier to reason
  about than many overlapping strategies.
- Use `try_insert_if_missing(...)` for obvious delimiters and separators.
- Use `unwanted(...)` for syntax you can consume and report without losing the
  rest of the parse.
- Add notes and help that explain why input is invalid, not only where it failed.

## Example command

```bash
cargo run --example json -- tests/data/json1.json
```

This example prints diagnostics and recovered output, demonstrating both strict
checking and graceful recovery.
