// AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use marser::capture;
use marser::parser::{Parser, SingleTokenParser};

fn main() {
    let p = capture!(bind!(SingleTokenParser::new('a'), x) => x);
    let _ = p.parse_str("a");
}
