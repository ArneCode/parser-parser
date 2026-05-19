# Release checklist (maintainer)

**AI assistance:** This document was drafted with AI assistance. The maintainer reviewed it. If anything looks wrong, please [open an issue](https://github.com/ArneCode/marser/issues/new).

Use this checklist when cutting a new version. **Publishing to crates.io is done locally by the maintainer** (not via CI).

## Before you publish

1. Ensure CI is green on `main` (see [.github/workflows/ci.yml](.github/workflows/ci.yml)).
2. Update [CHANGELOG.md](CHANGELOG.md) with the release date and notes.
3. Confirm packaging (no JSONTestSuite in the `marser` tarball):

   ```bash
   cargo package -p marser --list --allow-dirty | grep JSONTestSuite
   # (no output)
   ```

4. Run the full test matrix locally:

   ```bash
   cargo test --workspace
   cargo test -p marser --features "parser-trace json-testsuite"
   # requires: git submodule update --init tests/JSONTestSuite
   cargo clippy --workspace --all-features -- -D warnings
   ```

## Publish order (crates.io)

Publish **in this order**, waiting for each crate to appear on the index before the next (path dependencies resolve from the registry):

1. `cargo publish -p marser_macros`
2. `cargo publish -p marser-trace-schema`
3. `cargo publish -p marser`
4. `cargo publish -p marser-trace-viewer`

Pre-flight per crate (optional but recommended):

```bash
cargo publish -p <crate> --dry-run
```

**First time:** log in with `cargo login`, and claim the crate names on [crates.io](https://crates.io/) if needed.

## After publish

1. Tag the release: `git tag v0.1.0 && git push origin v0.1.0`
2. Create a GitHub release from the tag; paste the CHANGELOG section.
3. Verify [docs.rs/marser](https://docs.rs/marser) builds (docs.rs picks up new releases automatically; first build may take a few minutes).

## Experimental crates

`marser-trace-schema` and `marser-trace-viewer` are experimental: document breaking changes in CHANGELOG when trace formats or viewer APIs change.
