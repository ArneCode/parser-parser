# Core Concepts

This page explains the concepts behind `marser` in practical terms.

## Parser vs Matcher

From the library internals:

- `Matcher` is the matching interface (`src/matcher/mod.rs`)
- `Parser` is the parsing interface (`src/parser/mod.rs`)

In day-to-day grammar writing:

- Use matchers to describe *what must appear* in input.
- Use parsers to produce *what value you want back*.

`capture!` is the bridge: it runs matcher logic and builds parser output.

## `capture!`

`capture!` lets you:

- define a matching pattern
- bind values/spans from parts of that pattern
- return a typed output

This style is used in the tutorial page [Build a Simple JSON Parser](crate::guide::worked_json_example).

## `one_of(...)`

`one_of` is ordered choice: try alternatives left to right and take the first success.

Example pattern from JSON grammar style:

```rust,ignore
one_of((object, array, string, number, boolean, null))
```

## Lookahead

- `positive_lookahead(x)` checks that `x` matches next, without consuming input.
- `negative_lookahead(x)` checks that `x` does *not* match next, without consuming input.

Use lookahead to avoid ambiguous parses and to improve diagnostics.

## Repetition and optionality

- `many(x)` for zero or more
- `one_or_more(x)` for one or more
- `optional(x)` for optional segments

These are core building blocks for lists, whitespace, and token groups.

## Recovery and diagnostics

`marser` supports attaching error context and recovering to continue parsing:

- `add_error_info(...)` enriches furthest-failure diagnostics
- `recover_with(...)` can produce fallback output and continue
- helpers like `unwanted(...)` and `try_insert_if_missing(...)` are useful for user-facing errors
