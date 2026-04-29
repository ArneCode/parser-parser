use std::fs;
use std::path::{Path, PathBuf};

#[path = "../examples/mini_language/mod.rs"]
mod mini_language;

fn collect_ml_files(root: &Path) -> Vec<PathBuf> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|err| {
            panic!("failed to read directory {}: {err}", dir.display());
        });

        for entry in entries {
            let entry = entry.unwrap_or_else(|err| {
                panic!("failed to read directory entry in {}: {err}", dir.display());
            });
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "ml") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    visit(root, &mut files);
    files.sort();
    files
}

#[test]
fn valid_fixtures_parse_without_recovery_errors() {
    let root = Path::new("tests/data/mini_language/valid");
    let files = collect_ml_files(root);
    assert!(
        !files.is_empty(),
        "expected at least one valid mini_language fixture in {}",
        root.display()
    );

    for file in files {
        let source = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("failed to read fixture {}: {err}", file.display());
        });
        let parse_result = mini_language::parse_source(&source).unwrap_or_else(|err| {
            panic!(
                "valid fixture {} failed with hard parse error:\n{err:#?}",
                file.display()
            );
        });
        let (_, errors) = parse_result;
        assert!(
            errors.is_empty(),
            "valid fixture {} produced {} recovery diagnostic(s)",
            file.display(),
            errors.len()
        );
    }
}

#[test]
fn invalid_fixtures_produce_parse_diagnostics() {
    let root = Path::new("tests/data/mini_language/invalid");
    let files = collect_ml_files(root);
    assert!(
        !files.is_empty(),
        "expected at least one invalid mini_language fixture in {}",
        root.display()
    );

    for file in files {
        let source = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("failed to read fixture {}: {err}", file.display());
        });
        match mini_language::parse_source(&source) {
            Ok((_functions, errors)) => {
                assert!(
                    !errors.is_empty(),
                    "invalid fixture {} parsed without any diagnostics",
                    file.display()
                );
            }
            Err(_hard_error) => {
                // Hard parse failures are valid outcomes for invalid fixtures.
            }
        }
    }
}

#[test]
fn non_interactive_valid_fixtures_run_successfully() {
    let root = Path::new("tests/data/mini_language/valid");
    let files = collect_ml_files(root);

    for file in files {
        let source = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("failed to read fixture {}: {err}", file.display());
        });
        if source.contains("input(") {
            // Skip interactive fixtures in automated runtime checks.
            continue;
        }

        let (value, errors) = mini_language::run_source(&source).unwrap_or_else(|_err| {
            panic!(
                "non-interactive valid fixture {} failed to run",
                file.display()
            );
        });
        assert!(
            errors.is_empty(),
            "non-interactive valid fixture {} had {} diagnostic(s)",
            file.display()
            ,
            errors.len()
        );
        let _ = value;
    }
}
