# Contributing to marser

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

Thank you for your interest in **marser** — a PEG-style parser combinator library for Rust. This file explains how to work in the repository. For using the library in your own project, start with [README.md](README.md) and the [guide](guide/) / [docs.rs](https://docs.rs/marser).

---


## Ways to contribute

- **Bug reports** — [Open an issue](https://github.com/ArneCode/marser/issues/new) with a minimal repro, `marser` version, and Rust toolchain (`rustc -V`).
- **Feature ideas** — Open an issue first for larger API or behavior changes; smaller fixes can go straight to a PR.
- **Pull requests** — Fix a bug, improve docs/tests, or implement an agreed feature. See [Pull requests](#pull-requests).
- **Security** — Do **not** use public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).

There is no separate contributor license agreement; contributions are under the same [MIT license](LICENSE) as the project.

---

## Prerequisites

- **Rust 1.88+** (`rust-version` in `Cargo.toml`; stable **let chains** required).
- **git** with submodule support for the full test matrix.
- **Linux** (or WSL) is the primary dev environment; CI uses `ubuntu-latest`.

Clone and initialize the optional JSON corpus:

```bash
git clone https://github.com/ArneCode/marser.git
cd marser
git submodule update --init tests/JSONTestSuite
```

---

## Repository layout

This is a **Cargo workspace** at the repo root:

| Path | Crate | Role |
| ---- | ----- | ---- |
| `.` (root) | `marser` | Main library |
| `macros/` | `marser_macros` | `capture!` proc-macro |
| `marser-trace-schema/` | `marser-trace-schema` | Trace JSON schema (experimental) |
| `marser-trace-viewer/` | `marser-trace-viewer` | Trace TUI (experimental) |

User-facing narrative docs: [`guide/`](guide/) (embedded in rustdoc with the `embed-guide` feature). Integration tests and fixtures: [`tests/`](tests/) — see [tests/README.md](tests/README.md).

---

## Day-to-day development

### Quick check (most changes)

```bash
cargo test -p marser
cargo clippy -p marser --all-features -- -D warnings
```

### Full workspace

```bash
cargo test --workspace
cargo clippy --workspace --all-features -- -D warnings
```

### Optional feature matrix

```bash
cargo test -p marser --features "parser-trace json-testsuite"
cargo test -p marser --test capture_ui
cargo test -p marser-trace-schema
cargo test -p marser-trace-viewer
```

CI runs these on every push and pull request (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

### Examples

Examples require **`annotate-snippets`**:

```bash
cargo run --example json --features annotate-snippets -- tests/data/json1.json
```

### Docs (match docs.rs locally)

```bash
cargo doc -p marser --features parser-trace,annotate-snippets,embed-guide --no-deps --open
```

| Feature | When you need it |
| ------- | ---------------- |
| *(default)* | Core library only |
| `annotate-snippets` | Terminal diagnostics; **required for examples** |
| `parser-trace` | Tracing APIs and `trace_harness` tests |
| `json-testsuite` | `json_testsuite` integration test |
| `embed-guide` | Full `marser::guide` in rustdoc (on docs.rs by default) |

---

## Pull requests

1. **Fork / branch** — Use a descriptive branch name (`fix/recovery-duplicate-error`, `doc/capture-binds`).
2. **Keep scope focused** — One logical change per PR when possible.
3. **Run checks** — At minimum `cargo test -p marser` and `cargo clippy --workspace --all-features -- -D warnings`. Match what CI runs before asking for review.
4. **Changelog** — For user-visible API, feature, or behavior changes, add a bullet under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md) (right section: Added / Changed / Fixed / …). Skip CI-only, internal refactors, and test-only changes.
5. **Describe the PR** — What changed, why, and how you tested it. Link related issues.

Maintainers may request changes or squash-merge; there is no strict commit-message format, but clear messages help.

---

## `capture!` compile tests (trybuild)

Macro diagnostics are locked with [trybuild](https://docs.rs/trybuild) in `tests/capture_ui.rs` and `tests/ui/*.stderr`.

```bash
cargo test -p marser --test capture_ui
```

After a Rust upgrade or intentional diagnostic text changes:

```bash
TRYBUILD=overwrite cargo test -p marser --test capture_ui
```

Review diffs carefully, then re-run without `TRYBUILD=overwrite`.

---

## Style and conventions

- Follow existing patterns in the module you touch (naming, error types, `capture!` usage).
- **Clippy** — Workspace builds with `cargo clippy --workspace --all-features -- -D warnings`; fix or justify new warnings.
- **Formatting** — Use `cargo fmt --all` before submitting unless the diff would be unrelated noise.
- **Public API** — Document new items with `///` / `//!`; the main crate uses `#![deny(missing_docs)]`.
- **Experimental tracing** — `marser-trace-schema` and `marser-trace-viewer` may break between releases; note that in CHANGELOG when relevant.

If you use AI tools to draft code, ensure it is correct and tested; the maintainer is responsible for merged content. Some files in the repo mark AI-assisted authorship explicitly.

---

## Benchmarks and profiling (optional)

```bash
cargo bench --bench json_parse
```

Profiling script (needs `perf` / flamegraph tooling): [`scripts/flamegraph-json-parse.sh`](scripts/flamegraph-json-parse.sh). Details in [README.md](README.md).

---

## Releases and security (maintainers)

- **Cutting a release** — [RELEASE.md](RELEASE.md) (crates.io order, tagging, packaging checks).
- **Security reports** — [SECURITY.md](SECURITY.md).

Contributors do not need those steps for normal PRs.

---

## Questions

If something in this guide is unclear or out of date, open an issue or note it in your PR. For design questions on error recovery, `capture!`, or tracing, the [guide](guide/) and [examples/](examples/) are the best on-ramp after the README.
