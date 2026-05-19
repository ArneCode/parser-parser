//! Shared JSON fixture loading and grammar for the `json_parse` Criterion bench and the
//! `profile_json_parse` profiling binary.

use std::path::{Path, PathBuf};

use marser::parser::Parser;

#[path = "../examples/json/grammar.rs"]
#[allow(dead_code)]
mod json_grammar;

pub use json_grammar::JsonValue;
pub use json_grammar::get_json_grammar;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fixture {
    Json0,
    Canada,
}

impl Fixture {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Json0 => "parse_json0",
            Self::Canada => "parse_canada",
        }
    }

    pub fn path(self) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        match self {
            Self::Json0 => root.join("tests/data/json0.json"),
            Self::Canada => root.join("benches/data/canada.json"),
        }
    }
}

pub fn load_src(fixture: Fixture) -> String {
    let path = fixture.path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// One successful parse and the same assertions as the Criterion bench.
pub fn assert_parse_clean<'src, P>(label: &str, parser: &P, src: &'src str)
where
    P: Parser<'src, &'src str, Output = JsonValue<'src>> + Clone,
{
    let (value, errors) = parser
        .parse_str(src)
        .unwrap_or_else(|_| panic!("{label}: hard parse error"));
    assert!(
        errors.is_empty(),
        "{label}: expected no recovery diagnostics, got {} diagnostic(s)",
        errors.len()
    );
    std::hint::black_box(value);
}
