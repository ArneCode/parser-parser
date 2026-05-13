use marser::capture;

fn main() {
    let _p = capture!(
        (
            bind!(marser::parser::SingleTokenParser::new('a'), x as i32),
            bind!(marser::parser::SingleTokenParser::new('b'), x as u64)
        ) => x
    );
}
