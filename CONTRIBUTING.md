# Contributing to marser

## Feature flags

This repository uses **`parser-erased`** for normal development and CI-style checks (type-erased parser values).

**Examples** (`json`, `mini_language`, `mini_script`) are declared with `required-features = ["annotate-snippets", "parser-erased"]`. Build or run them with both features:

```bash
cargo run -p marser --features "parser-erased annotate-snippets" --example json -- tests/data/json1.json
```

For library-only checks (no examples, no pretty `eprint` APIs in the build), `parser-erased` alone is enough:

```bash
cargo check --features parser-erased
cargo test --features parser-erased
```

Full rustdoc for all optional surface (including tracing and annotate-snippets):

```bash
cargo doc --features parser-erased,parser-trace,annotate-snippets --no-deps
```

To also exercise tracing in your own runs, add **`parser-trace`** where needed.

## Macro compile tests (`trybuild`)

The integration test `tests/capture_ui.rs` compiles fixtures under `tests/ui/` and compares compiler diagnostics to checked-in `*.stderr` files.

Run only these tests:

```bash
cargo test --features parser-erased --test capture_ui
```

After upgrading the Rust toolchain (or when diagnostic text changes but macro behavior is still correct), regenerate golden stderr from the repository root:

```bash
TRYBUILD=overwrite cargo test --features parser-erased --test capture_ui
```

Review diffs, commit intentional changes, then re-run **without** `TRYBUILD=overwrite` to confirm the suite passes.

## Docs

- Crate guide sources live under `guide/` and are pulled into rustdoc via `src/guide.rs`.
- The project README is included as the `marser` crate front page (`src/lib.rs`).

For more context, see the **For contributors: compile tests (`trybuild`)** section in `README.md`.
