# marser

this is a parser-combinator library for PEG Grammars centered on being able to write grammars in a natural way inside rust code, with a focus on good error messages. I plan to upload this to crates.io once it's more polished.


## Example

A full JSON grammar example is available at `examples/json.rs` and can be run with:

```bash
cargo run -p marser --example json -- <path-to-json-file>
```

An example of error messages: 

Parser run with the following input:

```json
{
    "foo": 123,
    "bar": [1, ,2 ,.3
}
```

Produces the following error message:

![alt text](image-1.png)

It also produces a recovered AST:
```json
{
    "foo": 123,
    "bar": [
        1,
        2,
        invalid
    ]
}
```
(this is the the result AST converted back to a string)

you can try this yourself by running the following command:

```bash
cargo run -p marser --example json -- tests/data/json1.json
```

## Macros

Grammars are usually written with the `capture!` procedural macro. Import it from the main crate:

```rust
use marser::capture;
```

The `marser` package depends on `marser_macros` internally; you do not need a separate `marser_macros` dependency for normal use.

## Compile tests (`trybuild`)

The integration test `tests/capture_ui.rs` uses [trybuild](https://docs.rs/trybuild): it compiles small programs under `tests/ui/` that exercise `capture!` / `bind!`. **Pass** cases must build; **compile-fail** cases must fail with stderr matching the checked-in `tests/ui/*.stderr` files.

**Run only these tests** (with the same feature flag used elsewhere in this repo):

```bash
cargo test --features parser-erased --test capture_ui
```

**Regenerate golden stderr** after a Rust toolchain upgrade (or whenever diagnostics change but the macro behavior is still correct). From the repo root:

```bash
TRYBUILD=overwrite cargo test --features parser-erased --test capture_ui
```

Then review the diffs to `tests/ui/*.stderr` (and any updated pass fixtures), commit what you intend to keep, and re-run the command **without** `TRYBUILD=overwrite` to confirm the suite is green.

If you are new to this library, start with the beginner guide:

- `guide/README.md` (index)
- `guide/01-quickstart.md` (includes parser vs matcher mental model)

## License

This project is licensed under the MIT License.
