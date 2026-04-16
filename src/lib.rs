use std::{collections::HashMap, rc::Rc};

use macros::capture;

use crate::grammar::{
    capture::{Capture, bind_result, bind_span},
    error::FurthestFailError,
    label::WithLabel,
    matcher::{
        Matcher, commit_matcher::commit_on, multiple::many, one_of::one_of,
        one_or_more::one_or_more, optional::optional,
    },
    parser::{Parser, deferred::recursive, token_parser::TokenParser},
};
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn serialize(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Self::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.serialize()).collect();
                format!("[{}]", items.join(","))
            }
            Self::Object(obj) => {
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, v.serialize()))
                    .collect();
                format!("{{{}}}", pairs.join(","))
            }
        }
    }
}

pub mod grammar;

fn get_json_grammar() -> impl Parser<char, Output = JsonValue> {
    recursive(|element| {
        let element = Rc::new(element.with_label("element"));
        let ws = Rc::new(many(one_of((' ', '\t', '\n', '\r'))));

        // --- Primitives ---

        let null = capture!(("null", ws.clone())  => JsonValue::Null );

        let bool_false = capture!(("false", ws.clone())  => JsonValue::Boolean(false) );
        let bool_true = capture!(("true", ws.clone())  => JsonValue::Boolean(true) );
        let boolean = one_of((bool_true, bool_false));

        // Simplified number parser (matches digits)
        let number = capture!((
            optional(bind!('-', *digits)),
            one_of((
                bind!('0', *digits),
                (
                    bind!('1'..='9', *digits),
                    many(bind!('0'..='9', *digits))
                )
            )),
            optional((
                bind!('.', *digits), one_or_more(bind!('0'..='9', *digits))
            )),
            optional((
                bind!(one_of(('e', 'E')), *digits),
                optional(bind!(one_of(('+', '-')), *digits)),
                one_or_more(bind!('0'..='9', *digits))
            )),
            ws.clone()
        )
         => {
            let s: String = digits.into_iter().collect();
            JsonValue::Number(s.parse().unwrap_or(0.0))
        });

        let character = Rc::new(TokenParser::new(
            |c| *c != '"' && *c != '\\' && (*c as u32) >= 0x20,
            |x| *x,
        ));
        let hex_digit = Rc::new(one_of(('0'..='9', 'a'..='f', 'A'..='F')));
        let escaped_char = Rc::new(capture!({
            (
                '\\',
                bind!(one_of(('\"', '\\', '/', 'b', 'f', 'n', 'r', 't')), esc)
            )
        } => {
            match esc {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => '\u{0008}',
                'f' => '\u{000C}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => esc,
            }
        }));
        let unicode_escape = Rc::new(capture!({
            (
                '\\', 'u',
                bind!(hex_digit.clone(), *digits),
                bind!(hex_digit.clone(), *digits),
                bind!(hex_digit.clone(), *digits),
                bind!(hex_digit.clone(), *digits)
            )
        } => {
            let hex: String = digits.into_iter().collect();
            let codepoint = u32::from_str_radix(&hex, 16).unwrap_or(0xFFFD);
            std::char::from_u32(codepoint).unwrap_or('\u{FFFD}')
        }));
        let raw_string = Rc::new(capture!({
                commit_on('"',(
                many(one_of((
                    bind!(character.clone(), *chars),
                    bind!(escaped_char.clone(), *chars),
                    bind!(unicode_escape.clone(), *chars),
                ))),
                '"',
                ws.clone()
            ))
        } =>  {
            chars.into_iter().collect::<String>()
        }));

        // --- Complex Types (Recursive) ---

        // Array: [ element, element, ... ]
        let array = capture!({
                commit_on((ws.clone(), '['), (ws.clone(),
                optional((bind!(element.clone(), *elements),
                many((
                    ',', ws.clone(),
                    bind!(element.clone(), *elements)
                )))),
                ']'.with_label("]"), ws.clone()
            ))
        } =>  {
            JsonValue::Array(elements)
        });

        // Object: { "key": value, ... }
        let object = capture!({

                commit_on((ws.clone(), '{'.with_label("{")),
                (
                ws.clone(),
                optional((
                    bind!(raw_string.clone(), *keys), ':', ws.clone(),
                    bind!(element.clone(), *values)
                )),
                many(
                    commit_on(
                        ','.with_label(","), (
                            ws.clone(),
                            (
                                bind!(raw_string.clone(), *keys), ':', ws.clone(),
                                bind!(element.clone(), *values)
                            ).with_label("key-value pair")
                        )
                    ).add_error_info(
                        capture!((
                            bind_span!(",",comma),
                            ws.clone(),
                            '}') => move |err: &mut FurthestFailError|{
                            err.add_extra_label(comma,"trailing comma",ariadne::Color::Blue);
                        }),
                    ),
                ),
                '}'.with_label("}"), ws.clone()
                )
                )
        } => {
            let map: HashMap<String, JsonValue> = keys.into_iter().zip(values).collect();
            JsonValue::Object(map)
        });

        // --- Final Dispatcher ---
        // We combine all possible JSON types into one parser
        one_of((
            object,
            array,
            capture!( bind!(raw_string.clone(), s)  => JsonValue::String(s)),
            number,
            boolean,
            null,
        ))
    })
}
#[cfg(test)]
mod tests {
    use crate::grammar::parse;

    use super::*;
    use std::{env, fs, os::unix::fs::FileExt, path::Path};

    fn run_jsonsuite_case(
        grammar: &impl Parser<char, Output = JsonValue>,
        path: &Path,
    ) -> Option<String> {
        let file_name = path
            .file_name()
            .expect("JSON test file has no name")
            .to_string_lossy()
            .into_owned();

        if !(file_name.starts_with("y_")
            || file_name.starts_with("n_")
            || file_name.starts_with("i_"))
        {
            return None;
        }

        let raw_content = fs::read(path).expect("Failed reading JSON test file");
        let content = match std::str::from_utf8(&raw_content) {
            Ok(content) => content.to_owned(),
            Err(_) => {
                if file_name.starts_with("y_") {
                    return Some(format!("Expected valid JSON test to be UTF-8: {file_name}"));
                }
                if file_name.starts_with("n_") {
                    return None;
                }
                return Some(format!("Informational test is not UTF-8: {file_name}"));
            }
        };

        let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result: Result<
                (JsonValue, Vec<grammar::error::ParserError>),
                grammar::error::FurthestFailError,
            > = parse(grammar, &content);
            result
        }));

        if file_name.starts_with("y_") {
            match parse_result {
                Ok(Ok(_)) => None,
                Ok(Err(err)) => {
                    err.eprint_ariadne(&file_name, &content);
                    Some(format!("Expected valid JSON test to pass: {file_name}"))
                }
                Err(_) => Some(format!("Parser panicked on valid JSON test: {file_name}")),
            }
        } else if file_name.starts_with("n_") {
            let accepted = matches!(parse_result, Ok(Ok(_)));
            if accepted {
                Some(format!("Expected invalid JSON test to fail: {file_name}"))
            } else {
                None
            }
        } else {
            None
        }
    }

    #[test]
    fn test_standard_suite_direct() {
        let grammar = get_json_grammar();
        let suite_dir = "./tests/JSONTestSuite/test_parsing";
        let mut paths: Vec<_> = fs::read_dir(suite_dir)
            .expect("Missing JSONTestSuite. Run: git clone https://github.com/nst/JSONTestSuite.git tests/JSONTestSuite")
            .filter_map(Result::ok)
            .collect();
        paths.sort_by_key(|entry| entry.file_name());

        let mut accepted_info = 0usize;
        let mut rejected_info = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for entry in paths {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !(file_name.starts_with("y_")
                || file_name.starts_with("n_")
                || file_name.starts_with("i_"))
            {
                continue;
            }
            if file_name.contains("_structure") {
                continue;
            }
            if file_name.starts_with("i_") {
                let raw_content = fs::read(entry.path()).unwrap();
                let accepted = std::str::from_utf8(&raw_content)
                    .ok()
                    .and_then(|content| {
                        let parse_result: Result<
                            (JsonValue, Vec<grammar::error::ParserError>),
                            grammar::error::FurthestFailError,
                        > = parse(&grammar, content);
                        Some(parse_result.is_ok())
                    })
                    .unwrap_or(false);
                if accepted {
                    accepted_info += 1;
                } else {
                    rejected_info += 1;
                }
                continue;
            }
            if let Some(failure) = run_jsonsuite_case(&grammar, &entry.path()) {
                failures.push(failure);
            }
        }

        println!("JSONTestSuite i_* results: accepted={accepted_info}, rejected={rejected_info}");
        assert!(
            failures.is_empty(),
            "JSONTestSuite mismatches ({}):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn test_standard_suite_single_file_from_env() {
        let path =
            env::var("JSONSUITE_FILE").expect("Set JSONSUITE_FILE to run one JSONTestSuite case");
        let grammar = get_json_grammar();
        if let Some(failure) = run_jsonsuite_case(&grammar, Path::new(&path)) {
            panic!("{failure}");
        }
    }
}
