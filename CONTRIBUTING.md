# Contributing to marser

## Feature flags

The **`embed-guide`** feature is **not** needed for `cargo build` / `cargo test`; enable it for local rustdoc when you want the long-form `marser::guide` chapters (see **Docs** below).

**Examples** (`json`, `mini_language`, `mini_script`) are declared with `required-features = ["annotate-snippets"]`. Build or run them with:

```bash
cargo run -p marser --features annotate-snippets --example json -- tests/data/json1.json
```

For library-only checks (no examples, no pretty `eprint` APIs in the build):

```bash
cargo check
cargo test
```

Full rustdoc for all optional surface (including tracing, annotate-snippets, and the long-form **`marser::guide`** chapters plus the README as the crate front page):

```bash
cargo doc --features parser-trace,annotate-snippets,embed-guide --no-deps
```

Without **`embed-guide`**, `cargo doc` still builds, but the embedded book chapters and full README-in-rustdoc are skipped for faster local builds; use the command above when you need the same content as [docs.rs](https://docs.rs/marser).

To also exercise tracing in your own runs, add **`parser-trace`** where needed.

## Macro compile tests (`trybuild`)

The integration test `tests/capture_ui.rs` compiles fixtures under `tests/ui/` and compares compiler diagnostics to checked-in `*.stderr` files.

Run only these tests:

```bash
cargo test --test capture_ui
```

After upgrading the Rust toolchain (or when diagnostic text changes but macro behavior is still correct), regenerate golden stderr from the repository root:

```bash
TRYBUILD=overwrite cargo test --test capture_ui
```

Review diffs, commit intentional changes, then re-run **without** `TRYBUILD=overwrite` to confirm the suite passes.

## Docs

- Crate guide sources live under `guide/` and are pulled into rustdoc via **`src/guide_embed.rs`** when the **`embed-guide`** Cargo feature is enabled (including on docs.rs).
- Without `embed-guide`, `marser::guide` is a small stub module; the README is not embedded as the crate root doc until you enable `embed-guide`.
- For the full rustdoc experience locally, use `embed-guide` as in the `cargo doc` example in **Feature flags** above.

For more context, see the **For contributors: compile tests (`trybuild`)** section in `README.md`.

## Benchmarks and flamegraphs

- **`cargo bench --bench json_parse`** — Criterion benches for the JSON demo grammar (`benches/json_parse.rs`, fixtures under `tests/data/` and `benches/data/`).
- **`./scripts/flamegraph-json-parse.sh`** — runs `cargo flamegraph` with a `PERF` hint for Ubuntu/WSL generic `perf` when needed. Requires `cargo install flamegraph` and a working `perf` (WSL: often `linux-tools-generic`; see `README.md`).

Bench profile enables debug symbols via **`[profile.bench] debug = true`** in the crate `Cargo.toml` for readable stacks.
