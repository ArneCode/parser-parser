use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use marser::parser::Parser;

#[path = "json_parse_shared.rs"]
mod shared;

use shared::{Fixture, assert_parse_clean, get_json_grammar, load_src};

fn bench_fixture(c: &mut Criterion, fixture: Fixture) {
    let label = fixture.label();
    let src = load_src(fixture);
    let parser = get_json_grammar();

    assert_parse_clean(label, &parser, src.as_str());

    let mut group = c.benchmark_group(label);
    if fixture.uses_extended_criterion_timing() {
        // Default is ~100 samples in ~5s; full-file parse is hundreds of ms per iter.
        group
            .measurement_time(Duration::from_secs(20))
            .sample_size(50);
    }
    group.bench_function("parse", |b| {
        b.iter(|| {
            let out = parser.parse_str(black_box(src.as_str()));
            let _ = black_box(out);
        });
    });
    group.finish();
}

fn parse_fixtures(c: &mut Criterion) {
    for fixture in Fixture::ALL {
        bench_fixture(c, fixture);
    }
}

criterion_group!(benches, parse_fixtures);
criterion_main!(benches);
