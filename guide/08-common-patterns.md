# Common patterns

Short recipes experienced users often look for when sketching a grammar.

## Whitespace

Treat runs of ignorable tokens as a **matcher** you clone into sequences:

```rust
use std::rc::Rc;
use marser::matcher::multiple::many;
use marser::one_of::one_of;

let ws = Rc::new(many(one_of((' ', '\t', '\n', '\r'))));
// use ws.clone() before/after tokens where flexibility helps
# let _ = ws;
```

Keep whitespace **out** of token parsers when you want keywords to match exactly at boundaries; pull `ws` into the surrounding rule instead.

## Comma-separated lists

Typical shape: optional first element, then zero or more `,` + element repetitions. Inside `capture!`, use **repeated binds** (`*items`) for each `bind!(..., *items)` that appends.

Sketch:

```text
[ optional(elem (, elem)*) ]
```

See [Build a Simple JSON Parser](crate::guide::worked_json_example) for arrays/objects, and `examples/json/grammar.rs` for recovery-aware commas.

## Distinctive-prefix commits

Use `commit_on(prefix, rest)` after the parser has seen a prefix that clearly picks one construct. Good commit points are:

- an opening delimiter such as `(`, `{`, `[`
- a keyword such as `let`, `if`, `while`
- a prefix that would produce misleading backtracking if another branch were tried

Avoid committing on tokens that several constructs still share. The goal is: **backtrack while still deciding what the user meant; commit once you know the construct**.

## Recursive rules

Use `recursive` whenever a rule refers to itself (JSON `value`, expressions, blocks).
Annotating the `DeferredWeak` input (here for `&str` parsers) is enough for type
inference; a real grammar would clone that handle into nested rules instead of
ignoring it:

```rust
use marser::capture;
use marser::matcher::{any_token::AnyToken, negative_lookahead::negative_lookahead};
use marser::parser::{recursive, DeferredWeak, Parser};

let _value = recursive(|_weak: DeferredWeak<'_, '_, &str, ()>| {
    capture!(negative_lookahead(AnyToken) => ())
});
```

## Invalid or fallback nodes

After a **committed** construct fails hard, you can still **recover** with `recover_with` so the surrounding parse continues and the AST records an explicit `Invalid` / error node. Pair with `commit_on` so “wrong top-level alternative” stays soft, but “inside this `{` we are parsing an object” stays strict.

See [Errors and Recovery](crate::guide::errors_and_recovery) and `examples/mini_script.rs`.

## Borrowed text vs owned values

Use the bind form that matches the boundary you want:

- `bind!(parser, value)` when you want the parser's semantic output
- `bind_span!(matcher, span)` when you only need a location
- `bind_slice!(matcher, text)` when you want a borrowed slice of the original input

`bind_slice!` is a good default for identifiers, raw literals, and invalid fragments that should preserve spelling. Convert to owned data only when later code actually needs ownership or normalization.

## Full-input parsing

`Parser::parse_str` / `parse_whole_input` use the same driver as `marser::parse`: the grammar is wrapped so **no trailing tokens** remain. Use `negative_lookahead(AnyToken)` patterns inside the library’s default wrapper; for sub-parsers that only parse a fragment, use segment-specific rules instead of the whole-input entrypoint.

## Type size and `maybe_erase_types`

Large `one_of` / nested `capture!` types compile to deep generic trees. When inference or compile time hurts, call `.maybe_erase_types()` on heavy parsers (requires Cargo feature **`parser-erased`**). Repository examples use this on some JSON sub-rules.

**Compile time vs runtime:** erasure uses dynamic dispatch and can shrink type-checking and codegen cost for huge grammars. If you only need erasure for developer builds, use `erase_types()` under `#[cfg(debug_assertions)]` and keep concrete types in release (as in `parse-rosetta-rs`’s `marser-app`), or add a **`fast-compile`** (or similar) Cargo feature on your grammar crate that enables erasure in release when you are not benchmarking.

**Measuring:** use `cargo build -p your_crate --timings` for a crate-level breakdown; on nightly, `cargo llvm-lines --release -p your_crate` shows monomorphization hotspots.

## When to stop reading recipes

This page is for quick reminders. When a pattern starts interacting with diagnostics, recovery, or tracing:

- go to [Errors and Recovery](crate::guide::errors_and_recovery) for committed rules and collected errors
- go to [Capture and Binds](crate::guide::capture_and_binds) for bind-shape pitfalls
- go to `examples/json/grammar.rs` or `examples/mini_script.rs` for full, runnable grammar layouts
