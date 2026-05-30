# marser

`marser` is a parser-combinator library for writing **PEG-style grammars in Rust** with a focus on useful errors, error recovery and good performance.

It supports: 
- **Zero-copy parsing** for faster parsers
- **Multiple input formats** - use `&str` and `&[u8]` / `&[T]` or implement the `Input` trait yourself.
- **Packrat-style caching** - just wrap your parsers in `.memoized()` to cache results at each position.
- **Simple debugging** of your parsers using a custom TUI

## Quickstart

To add this library to your Rust project run:
```bash
cargo add marser
```

This library has a couple of optional features. You can find them [below](#cargo-features).

## Example
This example parses dice notation like `2d6` into a struct:

```rust
use marser::capture;
use marser::matcher::one_or_more;
use marser::parser::Parser;

// the struct we want to parse into
#[derive(Debug, PartialEq)]
struct Roll {
    count: u32,
    sides: u32,
}

// A parser that can parse a number
fn number<'src>() -> impl Parser<'src, &'src str, Output = u32> + Clone {
    // capture defines a parser. It consists of a matcher (the part before `=>`) 
    // and a Rust expression that builds the output value (the part after `=>`).
    capture!(
        bind_slice!( // bind_slice! stores the matched part of the input inside a variable
            one_or_more('0'..='9'), // matches any sequence of digits
            number_slice as &'src str // the matched digits are available as `number_slice` of type `&'src str`
        )
        => // we can then define how to build the output value from the bound variables
            number_slice // we use the captured slice of digits
                .parse() // and parse it into a u32
                .expect("matched only digits")
    )
}

// A parser that can parse a roll like `2d6`
fn roll<'src>() -> impl Parser<'src, &'src str, Output = Roll> + Clone {
    // we again define a parser with capture!, this time for the whole roll
    capture!(
        ( // we define a sequence by putting multiple matchers in a tuple
          // they are matched one after another
            bind!(number(), count), // first we expect a number. We use bind! to store its value in `count`
            'd', // then we expect the literal character 'd'
            bind!(number(), sides) // then we expect another number, which we store in `sides`
        )
        => // finally we define how to build the output value from the bound variables
            Roll { count, sides }
    )
}

fn main() {
    // we can then use this parser we defined to parse a string
    let (roll, _errors) = roll().parse_str("2d6").unwrap();
    assert_eq!(roll, Roll { count: 2, sides: 6 });
}
```
Runnable examples live under [`examples/`](examples/README.md) (see also [below](#examples-in-this-repository)).

## Learn more

- [Guide](https://docs.rs/marser/latest/marser/guide/index.html) on docs.rs
- [API documentation](https://docs.rs/marser)
- [crates.io](https://crates.io/crates/marser)

## Cargo features

| Feature                 | When you need it                                                                                                                                                                        |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| *(default)*             | Core library only.                                                                                                                                                                      |
| **`annotate-snippets`** | Enables rendering of error messages using the annotate-snippets crate                                                                                                                   |
| **`parser-trace`**      | **Experimental:** record parser traces to replay them in the trace viewer TUI. See the [tracing guide](https://docs.rs/marser/latest/marser/guide/tracing_and_debugging/index.html) and [`marser-trace-viewer/`](marser-trace-viewer/README.md). |


**Compatibility:** Releases follow semver for the **documented public API**. Everyday composition (`capture!`, matchers, errors) is intended to stay stable across minors; **tracing** and trace crates may evolve faster. Macro **expansion** details are not a stability guarantee — please use macros as APIs, not generated internals.

## Requirements

- **Rust 1.88 or later** 

## Examples in this repository

Examples need the **`annotate-snippets`** feature for rendering of errors

| Example                                                  | What it shows                                                                                                  |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [`examples/json/`](examples/json/)                       | A JSON parser with error recovery and custom error messages.                                                   |
| [`examples/mini_language.rs`](examples/mini_language.rs) | Small language: statements, operator precedence, functions etc. with error recovery and custom error messages. |

Run JSON from a git clone:

```bash
cargo run --example json --features annotate-snippets -- tests/data/json1.json
```

### Error output sample

Input:

```json
{
    "foo": 123,
    "bar": [1, ,2 ,3
}
```

Example diagnostic, rendered using **`annotate-snippets`**:

![Example parse error for invalid JSON Screenshot](https://raw.githubusercontent.com/ArneCode/marser/main/image.png)

This parser can also still produce a recovered output:

```json
{
    "foo": 123,
    "bar": [
        1,
        2,
        3
    ]
}
```

The json example also has tracing support, so parsing can be stepped through in the trace viewer. See screenshot below. Left side is the rust source code for the parser, right side is the file being parsed.
<img width="1126" height="531" alt="grafik" src="https://github.com/user-attachments/assets/e0ba9ef1-ca19-4f30-937d-279b718e58da" />


## Early release

**Early release:** `marser` is my first published Rust library. Feedback on the API, error messages, and docs is welcome — [open an issue](https://github.com/ArneCode/marser/issues/new)

## License

This project is licensed under the [MIT License](LICENSE).

## AI assistance

Parts of this repository were drafted or expanded with AI tools (guide, library docs, tests, macros, trace crates, examples, and parts of this README). The maintainer reviewed this material. If you spot a mistake, please [open an issue](https://github.com/ArneCode/marser/issues/new).
