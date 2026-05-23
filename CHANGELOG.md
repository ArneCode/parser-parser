# Changelog

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- improved error recovery of json example
- changed docs for trace viewer

### Fixed
- label messed up error recognition if inner parser returned error

## [0.1.2] - 2026-05-20

### Added
- README files under `examples/`, `examples/json/`, and `examples/mini_language/` with run commands and layout notes.

### Changed
- README: links (docs.rs, crates.io, LICENSE, CONTRIBUTING), trimmed AI/tracing sections, example table points at `examples/*/README.md`, general improvements.

### Removed
- The `mini_script` example and its unused fixtures under `tests/data/mini_script/`.

## [0.1.1] - 2026-05-19

### Changed

- docs.rs rustdoc includes scraped code from the `json`, `mini_language`, and `mini_script` examples on relevant API pages (`doc-scrape-examples`).

### Fixed

- docs.rs builds again: `doc_auto_cfg` (removed in Rust 1.92) is replaced with `doc_cfg` under the `docsrs` cfg.

## [0.1.0] - 2026-05-19

### Changed

- Minimum supported Rust version is **1.88** (stable let chains in `if`/`while`).
- CI runs a nightly `cargo doc` with `--cfg docsrs` and the same features as docs.rs so removed or renamed rustdoc features are caught before publish.

## [0.1.0] - 2026-05-19

### Added

- Initial public release of `marser`, `marser_macros`, `marser-trace-schema`, and `marser-trace-viewer`.
- PEG-style parser combinators with `capture!`, matcher-level backtracking, and error recovery.
- Optional `annotate-snippets` feature for terminal diagnostics (`ParserError::eprint`, etc.).
- Optional `parser-trace` feature and experimental trace schema / viewer crates.
- Optional `embed-guide` feature for long-form rustdoc chapters (`marser::guide`).
- Integration tests, JSON example grammar, and Criterion benchmarks in the repository.

### Notes

- Trace file formats and trace-related APIs may change in future releases; pin versions and read release notes when upgrading.
- Macro expansion details are not a stability guarantee; use `capture!` and documented helpers as the public API.

[0.1.1]: https://github.com/ArneCode/marser/releases/tag/v0.1.1
[0.1.0]: https://github.com/ArneCode/marser/releases/tag/v0.1.0