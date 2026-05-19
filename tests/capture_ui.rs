//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.
//!
//! `trybuild` checks for `capture!` / `bind!` expansion (repeated binds, conflicts, trailing tokens).
//!
//! Run: `cargo test --test capture_ui`  
//! Regenerate `tests/ui/*.stderr` after rustc diagnostic churn:  
//! `TRYBUILD=overwrite cargo test --test capture_ui`  
//! (See the **Compile tests (`trybuild`)** section in the repo `README.md`.)

#[test]
#[cfg(not(miri))]
fn capture_macro_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/capture_macro_smoke.rs");
    t.compile_fail("tests/ui/capture_sigil_mismatch.rs");
    t.compile_fail("tests/ui/capture_value_span_same.rs");
    t.compile_fail("tests/ui/capture_trailing_tokens.rs");
    t.compile_fail("tests/ui/capture_type_mismatch.rs");
    t.compile_fail("tests/ui/capture_use_binds_in_result.rs");
}
