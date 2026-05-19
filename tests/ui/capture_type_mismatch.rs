// AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use marser::capture;

fn main() {
    let _p = capture!(
        (
            bind!(marser::parser::SingleTokenParser::new('a'), x as i32),
            bind!(marser::parser::SingleTokenParser::new('b'), x as u64)
        ) => x
    );
}
