use marser::capture;

fn main() {
    let _p = capture!(bind!(
        marser::parser::SingleTokenParser::new('a'),
        x,
        y,
        z
    ) => x);
}
