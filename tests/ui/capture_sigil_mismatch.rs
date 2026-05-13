use marser::capture;

fn main() {
    let _p = capture!(
        (
            bind!(marser::parser::SingleTokenParser::new('a'), x),
            bind!(marser::parser::SingleTokenParser::new('b'), *x)
        ) => x
    );
}
