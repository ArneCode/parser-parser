use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use marser::parser::Parser;

#[path = "../examples/json/grammar.rs"]
#[allow(dead_code)]
mod json_grammar;

fn bench_fixture(c: &mut Criterion, label: &'static str, path: &Path) {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let parser = json_grammar::get_json_grammar();

    let (value, errors) = parser
        .parse_str(src.as_str())
        .unwrap_or_else(|_| panic!("{label}: hard parse error"));
    assert!(
        errors.is_empty(),
        "{label}: expected no recovery diagnostics, got {} diagnostic(s)",
        errors.len()
    );
    black_box(value);

    c.bench_function(label, |b| {
        b.iter(|| {
            let out = parser.parse_str(black_box(src.as_str()));
            let _ = black_box(out);
        });
    });
}

fn parse_fixtures(c: &mut Criterion) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    bench_fixture(
        c,
        "parse_json0",
        &root.join("tests/data/json0.json"),
    );
    bench_fixture(
        c,
        "parse_canada",
        &root.join("benches/data/canada.json"),
    );
}

criterion_group!(benches, parse_fixtures);
criterion_main!(benches);
