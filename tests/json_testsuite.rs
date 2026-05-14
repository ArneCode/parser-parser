//! [nst/JSONTestSuite](https://github.com/nst/JSONTestSuite) conformance harness for the JSON grammar in
//! [`examples/json/grammar.rs`](../examples/json/grammar.rs) (recovery-oriented; shared with the `json` example and benches).
//!
//! **Requires** the `json-testsuite` Cargo feature (and `parser-erased` for the JSON example):
//!
//! ```text
//! cargo test --features "parser-erased json-testsuite" --test json_testsuite
//! ```
//!
//! **Submodule:** `git submodule update --init tests/JSONTestSuite`
//!
//! ## Expectations
//!
//! - `y_*` — must parse with `parse_str` returning `Ok` and **no** recovery diagnostics.
//! - `n_*` — must not accept as clean JSON: either a hard `Err`, or `Ok` with **at least one**
//!   recovery diagnostic (never `Ok` with an empty diagnostic list).
//! - `i_*` — implementation-defined; we only require **no panic** and a bounded parse attempt.
//!
//! A few pathological `n_structure_*` fixtures are skipped here (extremely deep / wide bracket
//! ladders) so the default matrix stays fast and stable; treat those as manual stress checks.

#![cfg(feature = "json-testsuite")]
#![allow(dead_code, unused_imports)] // `examples/json/grammar.rs` is pulled in only for parsing, not serialization helpers

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use marser::parser::Parser;

#[path = "../examples/json/grammar.rs"]
mod json_example;

const SUITE_REL: &str = "tests/JSONTestSuite/test_parsing";

/// Stack for each fixture (deep nesting cases need more than the default test thread).
const PARSE_STACK: usize = 32 * 1024 * 1024;

/// Extremely large / adversarial inputs: run manually if you change memoization or structure.
const SKIP_FILES: &[&str] = &[
    "n_structure_100000_opening_arrays.json",
    "n_structure_open_array_object.json",
];

fn suite_dir() -> PathBuf {
    PathBuf::from(SUITE_REL)
}

fn assert_submodule_present() {
    let marker = suite_dir().join("y_object.json");
    assert!(
        marker.is_file(),
        "JSONTestSuite fixtures missing (expected {}).\n\
         Initialize the submodule:\n\
           git submodule update --init tests/JSONTestSuite",
        marker.display()
    );
}

fn list_suite_files() -> Vec<PathBuf> {
    let dir = suite_dir();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    paths
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SuiteKind {
    Yes,
    No,
    Impl,
}

fn classify(path: &Path) -> Option<SuiteKind> {
    let name = path.file_name()?.to_str()?;
    if name.starts_with("y_") {
        Some(SuiteKind::Yes)
    } else if name.starts_with("n_") {
        Some(SuiteKind::No)
    } else if name.starts_with("i_") {
        Some(SuiteKind::Impl)
    } else {
        None
    }
}

fn exercise_file(path: &Path) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<non-utf8 name>")
        .to_string();

    if SKIP_FILES.contains(&name.as_str()) {
        return;
    }

    let source = match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                // The example grammar parses `&str` (UTF-8). Skip fixtures that are not valid UTF-8
                // byte streams as files (binary strings, UTF-16, malformed sequences).
                return;
            }
        },
        Err(e) => panic!("{}: read: {e}", path.display()),
    };

    let kind = classify(path).unwrap_or_else(|| panic!("{}: unknown filename prefix", path.display()));

    thread::scope(|s| {
        thread::Builder::new()
            .stack_size(PARSE_STACK)
            .spawn_scoped(s, move || {
                let parser = json_example::get_json_grammar();
                let r = parser.parse_str(source.as_str());
                match kind {
                    SuiteKind::Yes => match r {
                        Ok((_v, errs)) => assert!(
                            errs.is_empty(),
                            "{name}: y_* fixture must parse without recovery diagnostics (got {} error(s))",
                            errs.len()
                        ),
                        Err(err) => panic!(
                            "{name}: y_* fixture must parse successfully, got hard error:\n{err:#?}"
                        ),
                    },
                    SuiteKind::No => match r {
                        Ok((_v, errs)) => assert!(
                            !errs.is_empty(),
                            "{name}: n_* fixture must not parse as clean JSON (Ok with empty diagnostics)"
                        ),
                        Err(_err) => {}
                    },
                    SuiteKind::Impl => {
                        let _ = r;
                    }
                }
            })
            .expect("spawn parse thread")
            .join()
            .expect("parse thread panicked");
    });
}

#[test]
fn nst_yes_files_parse_cleanly() {
    assert_submodule_present();
    for path in list_suite_files() {
        if classify(&path) != Some(SuiteKind::Yes) {
            continue;
        }
        exercise_file(&path);
    }
}

#[test]
fn nst_no_files_are_not_clean_accept() {
    assert_submodule_present();
    for path in list_suite_files() {
        if classify(&path) != Some(SuiteKind::No) {
            continue;
        }
        exercise_file(&path);
    }
}

#[test]
fn nst_impl_files_do_not_panic() {
    assert_submodule_present();
    for path in list_suite_files() {
        if classify(&path) != Some(SuiteKind::Impl) {
            continue;
        }
        exercise_file(&path);
    }
}

/// Run a single `test_parsing` file when `JSONSUITE_FILE` is set (used by `tests/run_jsonsuite_*.py`).
/// If the variable is unset, this test returns immediately so `cargo test --test json_testsuite`
/// can still run the matrix tests alone.
#[test]
fn nst_single_file_from_env() {
    assert_submodule_present();
    let Ok(path) = std::env::var("JSONSUITE_FILE") else {
        return;
    };
    let path = PathBuf::from(path);
    assert!(path.is_file(), "JSONSUITE_FILE={} is not a file", path.display());
    exercise_file(&path);
}
