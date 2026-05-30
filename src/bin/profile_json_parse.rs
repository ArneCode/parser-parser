//! Bounded-time JSON parse loop for sampling profilers (e.g. `cargo flamegraph`).
//!
//! For statistical benchmarks use Criterion instead:
//!
//! ```text
//! cargo bench --bench json_parse
//! ```
//!
//! Typical flamegraph (`[profile.profiling]`: debug symbols, LTO off for clearer stacks):
//!
//! ```text
//! cargo flamegraph --profile profiling --bin profile_json_parse
//! ```
//!
//! Optional arguments after `--` are fixture name then wall-clock seconds:
//!
//! ```text
//! cargo flamegraph --profile profiling --bin profile_json_parse -- json0 3
//! ```
//!
//! Fixtures: `json0` (small), `canada`, `twitter`, `citm_catalog` (simdjson-data).
//! Default: `canada` and `5` seconds.

use std::time::{Duration, Instant};

use marser::parser::Parser;

#[path = "../../benches/json_parse_shared.rs"]
mod shared;

use shared::{Fixture, assert_parse_clean, get_json_grammar, load_src};

fn usage() -> ! {
    eprintln!("usage: profile_json_parse [json0|canada|twitter|citm_catalog] [seconds]");
    std::process::exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let fixture_arg = args.next();
    let seconds_arg = args.next();
    if args.next().is_some() {
        usage();
    }

    let fixture = fixture_arg.as_deref().unwrap_or("canada");
    let fixture = Fixture::from_parse_name(fixture).unwrap_or_else(|| usage());

    let seconds: u64 = seconds_arg
        .as_deref()
        .map(|s| s.parse().unwrap_or_else(|_| usage()))
        .unwrap_or(5);

    let src = load_src(fixture);
    let parser = get_json_grammar();
    assert_parse_clean(fixture.label(), &parser, src.as_str());

    eprintln!(
        "profile_json_parse: fixture={} seconds={} (bounded wall time for perf/flamegraph)",
        fixture.label(),
        seconds
    );

    let budget = Duration::from_secs(seconds);
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        let out = parser.parse_str(std::hint::black_box(src.as_str()));
        let _ = std::hint::black_box(out);
    }
}
