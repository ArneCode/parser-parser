# marser

`marser` is a parser-combinator library for writing **PEG-style grammars in Rust**. Matchers describe structure, `capture!` turns matched input into typed values, and you can add **commit points**, **recovery**, and **user-facing diagnostics** without giving up readable grammar code.

## Why `marser`

- **Grammar stays close to the language**: ordered choice (`one_of`), repetition, lookahead, and sequencing compose the way you sketch a grammar on paper.
- **Typed output stays local to the rule**: `capture!` is the point where matcher-shaped grammar becomes your enum or AST node.
- **You choose when syntax becomes committed**: `commit_on(prefix, rest)` turns “wrong branch” soft failures into hard failures once the input clearly chose a construct.
- **Recovery is built in**: `recover_with`, `try_insert_if_missing`, `unwanted`, and `if_error` support partial ASTs and editor-style workflows where `Ok((output, errors))` can still carry diagnostics.

## Quickstart

Add this to your `Cargo.toml`:

```toml
[dependencies]
marser = { version = "0.1.0", features = ["parser-erased", "annotate-snippets"] }
```

`parser-erased` and `annotate-snippets` match how this repository runs examples. You can omit `annotate-snippets` if you do not need `ParserError::eprint` / `write`.

Then build a small parser. This example parses dice notation like `2d6` into a typed struct:

```rust
use marser::capture;
use marser::matcher::one_or_more;
use marser::parser::Parser;

#[derive(Debug, PartialEq)]
struct Roll {
    count: u32,
    sides: u32,
}

fn number<'src>() -> impl Parser<'src, &'src str, Output = u32> + Clone {
    capture!(
        bind_slice!(one_or_more('0'..='9'), n as &'src str)
            => n.parse().expect("matched only digits")
    )
}

fn roll<'src>() -> impl Parser<'src, &'src str, Output = Roll> + Clone {
    capture!(
        (bind!(number(), count), 'd', bind!(number(), sides))
            => Roll { count, sides }
    )
}

fn main() {
    let (roll, _errors) = roll().parse_str("2d6").unwrap();
    assert_eq!(roll, Roll { count: 2, sides: 6 });
}
```

What this shows:

- **`capture!` is the value boundary**: the grammar says what to consume; the expression after `=>` builds the Rust value.
- **`bind!` names a parser's output**: `bind!(number(), count)` runs `number()` and makes its result available as `count`.
- **`bind_slice!` borrows the matched text**: the digits are captured as a `&str` slice without any allocation.
- **Rules compose**: `roll()` is built from two `number()` parsers with a literal `'d'` between them.
- **Whole-input parse is the default**: `parse_str("2d6 extra")` fails because `marser` expects the grammar to consume all input.

Successful whole-input parse returns **`Ok((output, collected_errors))`**. `collected_errors` may be non-empty when recovery reported issues but still produced a value. A hard syntax failure returns **`Err(FurthestFailError)`**.

## Learn More

The longer guide is embedded in rustdoc on docs.rs as [`marser::guide`](https://docs.rs/marser/latest/marser/guide/index.html), and the same markdown lives under [`guide/`](guide/) in this repo.

Suggested next reads:

1. [Quickstart](https://docs.rs/marser/latest/marser/guide/quickstart/index.html) — dependency, mental model, first parser.
2. [Errors and Recovery](https://docs.rs/marser/latest/marser/guide/errors_and_recovery/index.html) — `Ok` vs `Err`, commits, recovery.
3. [Worked JSON](https://docs.rs/marser/latest/marser/guide/worked_json_example/index.html) or the repo examples below — realistic grammar shape.

## Cargo features (what to enable)

| Feature | When you need it |
|--------|-------------------|
| *(default)* | Core library only. |
| **`parser-erased`** | Use this when parser types become unwieldy; it enables `ParserCombinator::maybe_erase_types` and is used by this repo's examples. |
| **`annotate-snippets`** | Enables `ParserError::eprint`, `write`, and terminal-friendly diagnostics. **Examples in this repo require it.** |
| **`parser-trace`** | Structured parse tracing and trace files; companion crates are experimental (see below). |
| **`embed-guide`** | Embeds the long-form `marser::guide::*` chapters and the README into rustdoc. **Off by default** for faster `cargo build`; enabled on docs.rs. Use `cargo doc -p marser --features embed-guide` locally for the full book. |

**Compatibility:** Releases follow semver for the **documented public API**. Everyday composition (`capture!`, matchers, errors) is intended to stay stable across minors; **tracing** and trace crates may evolve faster. Macro **expansion** details are not a stability guarantee — use macros as APIs, not generated internals.

## Requirements

- **Rust 1.85 or later** (`rust-version` in `Cargo.toml`).

## Examples in this repository

Examples need **`parser-erased`** and **`annotate-snippets`** (see `Cargo.toml` `required-features`).

| Example | What it shows |
|--------|----------------|
| [`examples/json/`](examples/json/) | JSON demo: [`grammar.rs`](examples/json/grammar.rs) (recovery grammar, shared with tests/benches), [`main.rs`](examples/json/main.rs) (CLI, optional `parser-trace` / `--trace-file`). |
| [`examples/mini_script.rs`](examples/mini_script.rs) | Small language: statements, precedence, `commit_on` + `recover_with`. |
| [`examples/mini_language.rs`](examples/mini_language.rs) | Parse file + eval; recovered diagnostics vs fatal error. |

Run JSON from a clone:

```bash
cargo run -p marser --features "parser-erased annotate-snippets" --example json -- tests/data/json1.json
```

### Error output sample

Input:

```json
{
    "foo": 123,
    "bar": [1, ,2 ,.3
}
```

Example diagnostic (terminal; requires **`annotate-snippets`**):

![Example parse error for invalid JSON](image-1.png)

With recovery, the same run can still yield a partial AST (details in the JSON example and guide).

## Benchmarks and profiling

**Criterion** (`json_parse` bench) parses fixed fixtures (small `tests/data/json0.json` and `benches/data/canada.json` from [simdjson-data](https://github.com/simdjson/simdjson-data)):

```bash
cargo bench --bench json_parse
```

HTML output: `target/criterion/report/index.html`. Criterion may print `Gnuplot not found, using plotters backend`; install **`gnuplot`** (`sudo apt install gnuplot` on Debian/Ubuntu) if you want the Gnuplot backend—plotters works without it.

Bench builds use **`[profile.bench] debug = true`** in `Cargo.toml` so tools like **`cargo flamegraph`** get usable symbols. For a **short** profiling run (without waiting for Criterion’s full measurement schedule), use the **`profile_json_parse`** binary (defaults: `canada` fixture, 5 seconds of parsing):

```bash
cargo flamegraph --profile bench --bin profile_json_parse
# optional: cargo flamegraph --profile bench --bin profile_json_parse -- json0 3
```

Flamegraph still needs a working **`perf`** on Linux; on WSL2 the kernel-matched `linux-tools-*` package is often missing—try `sudo apt install linux-tools-common linux-tools-generic` and, if needed, `export PERF=/usr/lib/linux-tools/<version>-generic/perf`, or run [`scripts/flamegraph-json-parse.sh`](scripts/flamegraph-json-parse.sh) from this directory.

## Experimental tracing

The crates **`marser-trace-schema`** and **`marser-trace-viewer`** are **experimental**: APIs and on-disk formats may change. Pin versions and read release notes when upgrading. See the guide chapter [Tracing and Debugging](https://docs.rs/marser/latest/marser/guide/tracing_and_debugging/index.html).

## Macros

Grammars are usually written with the `capture!` procedural macro:

```rust
use marser::capture;
```

The `marser` crate depends on `marser_macros` internally; you do not need a separate `marser_macros` dependency for normal use.

---

## For contributors: compile tests (`trybuild`)

Short checklist: [Contributing](https://github.com/ArneCode/marser/blob/main/CONTRIBUTING.md).

The integration test `tests/capture_ui.rs` uses [trybuild](https://docs.rs/trybuild): programs under `tests/ui/` exercise `capture!` / `bind!`. **Pass** cases must build; **compile-fail** cases must match `tests/ui/*.stderr`.

```bash
cargo test --features parser-erased --test capture_ui
```

Regenerate golden stderr after a toolchain upgrade or intentional diagnostic changes:

```bash
TRYBUILD=overwrite cargo test --features parser-erased --test capture_ui
```

Review diffs, then re-run without `TRYBUILD=overwrite` until green.

## JSONTestSuite (optional)

The upstream [JSONTestSuite](https://github.com/nst/JSONTestSuite) corpus is wired as a **git submodule** at `tests/JSONTestSuite`. Fetch it with:

```bash
git submodule update --init tests/JSONTestSuite
```

The integration test `tests/json_testsuite.rs` is built only with the **`json-testsuite`** feature (together with **`parser-erased`** for the example grammar):

```bash
cargo test --features "parser-erased json-testsuite" --test json_testsuite
```

Per-file matrix helpers live under `tests/run_jsonsuite_*.py` (they set `JSONSUITE_FILE` and run `nst_single_file_from_env` in a subprocess).

## License

This project is licensed under the MIT License.
