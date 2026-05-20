# Examples

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

Runnable grammars for `marser` live under this directory. Each example is a Cargo `[[example]]` on the root `marser` crate (see [`Cargo.toml`](../Cargo.toml)).

Run commands from the **repository root**.

## Requirements

- **Rust 1.88+** (see the crate `rust-version`)
- **`annotate-snippets`** — both examples declare it as a `required-features` entry so they can print diagnostics with [`ParserError::eprint`](../src/error/mod.rs)

## Examples

| Directory / entrypoint | Cargo example name | What it shows |
| ---------------------- | ------------------ | ------------- |
| [`json/`](json/) | `json` | JSON parser with recovery, custom messages, optional tracing |
| [`mini_language/`](mini_language/) (CLI: [`mini_language.rs`](mini_language.rs)) | `mini_language` | Small language: functions, precedence, parse + eval, recovery |

## Run

```bash
# JSON (default path in the binary is tests/data/json1.json if you pass no file)
cargo run --example json --features annotate-snippets -- tests/data/json1.json

# Mini language
cargo run --example mini_language --features annotate-snippets -- tests/data/mini_language/valid/fibonacci.ml
```

See the README in each subdirectory for fixtures, CLI flags, and how the grammar is organized.

## Learn more

- [Guide: Build a Simple JSON Parser](https://docs.rs/marser/latest/marser/guide/worked_json_example/index.html) — tutorial-sized JSON (smaller than `json/grammar.rs`)
- [Guide: Errors and Recovery](https://docs.rs/marser/latest/marser/guide/errors_and_recovery/index.html)
- [Repository README](../README.md) — error screenshot and feature overview
