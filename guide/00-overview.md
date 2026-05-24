Where `marser` fits, trade-offs, and glossary.

<div class="warning">

**AI assistance:** This chapter was drafted with AI assistance while the library is still young. The guide is expected to improve over time as APIs and examples stabilize. If anything looks wrong or confusing, please [report it on GitHub](https://github.com/ArneCode/marser/issues/new).

</div>

# Overview

`marser` is a parser-combinator library for writing PEG-style grammars in Rust code.
It focuses on two things:

- expressing grammar rules in a readable, composable style
- producing useful error messages and recoverable results

## Where `marser` fits

**Good fit** when you want:

- hand-written grammars with **ordered choice** and explicit **lookahead**
- **PEG-style** composition in Rust (not a separate grammar file or codegen step)
- control over **when a branch becomes “committed”** so errors stay local and explainable
- **partial trees** and collected diagnostics for tooling (formatters, IDE-like flows)

**Trade-offs to be aware of**:

- You compose **Rust types**; very large grammars may need `erase_types()` or factoring to keep types manageable.
- The model is **combinator + macro** (`capture!`); learning bind shapes (`*`, `?`) matters for correctness.
- **Tracing** and the `marser-trace-*` crates are younger than the core parsing API; treat them as optional debugging infrastructure.

## What you will learn

In this guide, you will:

- build a small parser with `capture!` and combinators
- learn the difference between `Matcher` and `Parser`
- build a small but complete JSON parser from scratch
- understand error reporting and recovery patterns

## How to use this guide

- If you are new to parser combinators, start with [Quickstart](crate::guide::quickstart).
- If you want the mental model first, read [Core Concepts](crate::guide::core_concepts).
- If you learn best from building a complete grammar, jump to [Build a Simple JSON Parser](crate::guide::worked_json_example).
- If you are **evaluating** the library, see the reading order on the [guide index](crate::guide).

## Glossary

| Term | Meaning |
|------|--------|
| **Matcher** | Predicate / grammar fragment inside `capture!`: returns match success/failure and may bind into capture slots. |
| **Parser** | Produces a typed `Output` when it matches; what you usually store in `grammar` functions and call with `parse_str` / `parse_whole_input`. |
| **Soft failure** | No match at this position: parser returns `None`, matcher returns `false`; a sibling `one_of` branch may still apply. |
| **Hard failure** | `Err(FurthestFailError)` (or a signal that becomes one): the parse is not silently “try another rule” at this committed point. |
| **Committed parse** | After `commit_on(prefix, …)` matches `prefix`, failures in `rest` are treated as real errors for that construct. |
| **Recovery** | e.g. `recover_with`: catch a hard failure, rewind, run a fallback parser, and often record the original error in `collected_errors`. |
| **Inline error** | Diagnostics emitted via helpers like `try_insert_if_missing` / `unwanted` / `err_if_*`, surfaced as `ParserError::Inline` among collected errors. |

## Prerequisites

- Rust **1.88 or later** (this workspace’s documented MSRV; see the crate README / `rust-version` in `Cargo.toml`)
- Comfort with basic Rust syntax (functions, enums, pattern matching)
- Basic command line usage (`cargo run`, `cargo test`)

For **release policy** (semver, tracing, macro expansion), see the crate front
page for `marser` in the rendered docs. The project README is included there as
the crate-level documentation on docs.rs and in local `cargo doc` output.
