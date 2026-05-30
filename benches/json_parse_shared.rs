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
    Twitter,
    CitmCatalog,
}

#[allow(dead_code)] // `ALL` / timing helpers are bench-only; `parse_name` is profile-binary-only.
impl Fixture {
    pub const ALL: [Self; 4] = [Self::Json0, Self::Canada, Self::Twitter, Self::CitmCatalog];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Json0 => "parse_json0",
            Self::Canada => "parse_canada",
            Self::Twitter => "parse_twitter",
            Self::CitmCatalog => "parse_citm_catalog",
        }
    }

    /// Criterion groups for large simdjson-style files need longer measurement windows.
    pub const fn uses_extended_criterion_timing(self) -> bool {
        !matches!(self, Self::Json0)
    }

    pub fn path(self) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        match self {
            Self::Json0 => root.join("tests/data/json0.json"),
            Self::Canada => root.join("benches/data/canada.json"),
            Self::Twitter => root.join("benches/data/twitter.json"),
            Self::CitmCatalog => root.join("benches/data/citm_catalog.json"),
        }
    }

    pub fn parse_name(self) -> &'static str {
        match self {
            Self::Json0 => "json0",
            Self::Canada => "canada",
            Self::Twitter => "twitter",
            Self::CitmCatalog => "citm_catalog",
        }
    }

    pub fn from_parse_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "json0" => Some(Self::Json0),
            "canada" => Some(Self::Canada),
            "twitter" => Some(Self::Twitter),
            "citm_catalog" | "citm" => Some(Self::CitmCatalog),
            _ => None,
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
