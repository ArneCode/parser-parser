use std::{collections::HashMap, rc::Rc};

use macros::capture;

use crate::grammar::{
    capture::{Capture, bind_result, bind_span},
    error_handler::ParserError,
    label::WithLabel,
    matcher::{
        Matcher, commit_matcher::commit_on, multiple::many, one_of::one_of, optional::optional,
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
                bind!('.', *digits), many(bind!('0'..='9', *digits))
            )),
            optional((
                bind!(one_of(('e', 'E')), *digits),
                optional(bind!(one_of(('+', '-')), *digits)),
                many(bind!('0'..='9', *digits))
            ))
        )
         => {
            let s: String = digits.into_iter().collect();
            JsonValue::Number(s.parse().unwrap_or(0.0))
        });

        let character = Rc::new(TokenParser::new(|c| *c != '"' && *c != '\\', |x| *x));
        let raw_string = Rc::new(capture!({
            (
                '"',
                many(one_of((
                    bind!(character.clone(), *chars),
                    ('\\', bind!(one_of(('\"', '\\', '/', 'b', 'f', 'n', 'r', 't')), *chars)),
                ))),
                '"',
                ws.clone()
            )
        } =>  {
            chars.into_iter().collect::<String>()
        }));

        // --- Complex Types (Recursive) ---

        // Array: [ element, element, ... ]
        let array = capture!({
            (
                '[', ws.clone(),
                optional(bind!(element.clone(), *elements)),
                many((
                    ',', ws.clone(),
                    bind!(element.clone(), *elements)
                )),
                ']'.with_label("]"), ws.clone()
            )
        } =>  {
            JsonValue::Array(elements)
        });

        // Object: { "key": value, ... }
        let object = capture!({

                commit_on('{'.with_label("{"),
                (
                ws.clone(),
                optional((
                    bind!(raw_string.clone(), *keys), ':', ws.clone(),
                    bind!(element.clone(), *values)
                )),
                many(
                    commit_on(
                        ',', (
                            ws.clone(),
                            (
                                bind!(raw_string.clone(), *keys), ':', ws.clone(),
                                bind!(element.clone(), *values)).with_label("key-value pair"
                            )
                        )
                    ).add_error_info(
                        capture!((
                            bind_span!(",",comma),
                            ws.clone(),
                            '}') => move |err: &mut ParserError|{
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
    use std::fs;

    #[test]
    fn test_standard_suite() {
        let grammar = get_json_grammar();
        let paths = fs::read_dir("./tests/data").expect("Check test data path");

        for path in paths {
            let entry = path.unwrap();
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let content = fs::read_to_string(entry.path()).unwrap();
            // let chars = content.chars().collect::<Vec<_>>();

            // Prepare context and error handler (update these to match your lib)
            // let mut context = ParserContext::new(&chars);
            // let mut handler = EmptyErrorHandler;
            // let mut pos = 0;

            // let result = grammar.parse(&mut context, &mut handler, &mut pos);
            let result: Result<
                (JsonValue, Vec<grammar::error_handler::ParserError>),
                grammar::error_handler::ParserError,
            > = parse(&grammar, &content);

            match result {
                Ok((val, _)) => {
                    let serialized = val.serialize();
                    println!("File: {}, Serialized: {}", file_name, serialized);
                }
                Err(err) => {
                    err.eprint_ariadne(&file_name, &content);
                }
            }
            // match result {
            //     Ok(result) => ,
            //     Err(_) => todo!(),
            // }

            // if file_name.starts_with('y') {
            //     // Should pass
            //     assert!(result.is_ok(), "File {} should have passed", file_name);
            //     let val = result.unwrap().expect("Should return a value");

            //     // Round-trip test: Stringify and re-parse
            //     let serialized = val.serialize();
            //     // (Optionally re-parse 'serialized' here to ensure stability)
            // } else if file_name.starts_with('n') {
            //     // Should fail or return None
            //     let failed = result.is_err() || result.unwrap().is_none();
            //     assert!(failed, "File {} should have failed", file_name);
            // }
        }
    }
}
