use std::fs;
use std::path::Path;

#[path = "../examples/json.rs"]
mod json_example;

fn read_fixture(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path);
    })
}

#[test]
fn valid_json_parses_without_recovery_errors() {
    let valid_files = ["tests/data/json0.json"];

    for path in valid_files {
        let source = read_fixture(path);
        let parser = json_example::get_json_grammar();
        let (value, errors) = marser::parse(parser, source.as_str()).unwrap_or_else(|err| {
            panic!("valid fixture {} failed with hard parse error:\n{err:#?}", path);
        });

        assert!(
            errors.is_empty(),
            "valid fixture {} produced {} recovery diagnostic(s)",
            path,
            errors.len()
        );

        // Ensure AST is usable/serializable.
        let _serialized = value.serialize_pretty();
    }
}

#[test]
fn invalid_json_produces_recovery_errors_and_recovered_ast() {
    let invalid_files = [
        "tests/data/json1.json",
        "tests/data/json2.json",
        "tests/data/json3.json",
    ];

    for path in invalid_files {
        assert!(
            Path::new(path).exists(),
            "missing invalid fixture {}",
            path
        );
        let source = read_fixture(path);
        let parser = json_example::get_json_grammar();

        let parse_result = marser::parse(parser, source.as_str());
        let (value, errors) = parse_result.unwrap_or_else(|err| {
            panic!(
                "invalid fixture {} should recover into AST, but failed hard:\n{err:#?}",
                path
            );
        });

        assert!(
            !errors.is_empty(),
            "invalid fixture {} parsed without diagnostics",
            path
        );

        // Recovered AST should still be usable.
        let _serialized = value.serialize_pretty();
    }
}
