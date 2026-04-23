use std::{collections::HashMap, env, fs, process, rc::Rc};

use marser_macros::capture;

use marser::{
    error::{FurthestFailError, ParserError},
    label::WithLabel,
    matcher::{
        AnyToken, MatcherCombinator, commit_matcher::commit_on, multiple::many, negative_lookahead,
        one_or_more::one_or_more, optional::optional, positive_lookahead, unwanted::unwanted,
    },
    one_of::one_of,
    parser::{Parser, ParserCombinator, deferred::recursive, token_parser::TokenParser},
};

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Invalid,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn serialize(&self) -> String {
        match self {
            Self::Invalid => "invalid".to_string(),
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
    /// Public method for pretty-printed JSON
    pub fn serialize_pretty(&self) -> String {
        self.serialize_internal(0)
    }

    fn serialize_internal(&self, indent_level: usize) -> String {
        let indent_size = 4;
        let current_indent = " ".repeat(indent_level * indent_size);
        let nested_indent = " ".repeat((indent_level + 1) * indent_size);

        match self {
            Self::Invalid => "invalid".to_string(),
            Self::Null => "null".to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),

            Self::Array(arr) => {
                if arr.is_empty() {
                    return "[]".to_string();
                }
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| {
                        format!(
                            "{}{}",
                            nested_indent,
                            v.serialize_internal(indent_level + 1)
                        )
                    })
                    .collect();
                format!("[\n{}\n{current_indent}]", items.join(",\n"))
            }

            Self::Object(obj) => {
                if obj.is_empty() {
                    return "{}".to_string();
                }
                // Note: HashMap iteration order is random.
                // For deterministic output, you could collect and sort keys here.
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}\"{}\": {}",
                            nested_indent,
                            k,
                            v.serialize_internal(indent_level + 1)
                        )
                    })
                    .collect();
                format!("{{\n{}\n{current_indent}}}", pairs.join(",\n"))
            }
        }
    }
}

pub fn get_json_grammar<'src>() -> impl Parser<'src, &'src str, Output = JsonValue> {
    recursive(|element| {
        let ws = Rc::new(many(one_of((' ', '\t', '\n', '\r'))));

        let null = capture!(("null", ws.clone())  => JsonValue::Null );
        let bool_false = capture!(("false", ws.clone())  => JsonValue::Boolean(false) );
        let bool_true = capture!(("true", ws.clone())  => JsonValue::Boolean(true) );
        let boolean = one_of((bool_true, bool_false));

        let number = capture!(
            commit_on(positive_lookahead(one_of(('-', '.', '+', '0'..='9'))),
            bind_slice!((
                optional('-'),
                one_of((
                    '0',
                    ('1'..='9',many('0'..='9'))
                )),
                optional((
                    '.', one_or_more('0'..='9')
                )),
                optional((
                    one_of(('e', 'E')),
                    optional(one_of(('+', '-'))),
                    one_or_more('0'..='9')
                )),
                negative_lookahead(one_of(('+','-','0'..='9','.','e','E')))
            ), slice as &'src str))
             => {
                JsonValue::Number(slice.parse().unwrap_or(0.0))
            }
        )
        .add_error_info(one_of((
            capture!(
                (
                    optional('-'),
                    bind_span!('0', zero),
                    '0'..='9'
                )
                => Box::new(move |err: &mut FurthestFailError|{
                    err.add_extra_label(zero,"leading zero",ariadne::Color::Blue);
                    err.add_note("Leading zeros are not allowed in JSON numbers".to_string());
                }) as Box<dyn Fn(&mut FurthestFailError)>
            ),
            capture!(
                (
                    optional('-'),
                    bind_span!('.', dot),
                )
                => Box::new(move |err: &mut FurthestFailError|{
                    err.add_extra_label(dot,"missing integer part",ariadne::Color::Blue);
                    err.add_note("Floating point numbers need an integer part".to_string());
            }) as Box<dyn Fn(&mut FurthestFailError)>
            ),
        )))
        .recover_with(
            many(one_of(('+', '-', '0'..='9', '.', 'e', 'E'))),
            JsonValue::Invalid,
        );

        let character = Rc::new(TokenParser::new(
            |c| *c != '"' && *c != '\\' && (*c as u32) >= 0x20,
            |x| *x,
        ));
        let hex_digit = Rc::new(one_of(('0'..='9', 'a'..='f', 'A'..='F')));
        let escaped_char = capture!({
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
        });
        let unicode_escape = capture!({
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
        });
        let raw_string = Rc::new(
            capture!({
                commit_on(
                    '"',(
                    many(one_of((
                        bind!(character.clone(), *chars),
                        bind!(escaped_char, *chars),
                        bind!(unicode_escape, *chars),
                    ))),
                    '"',
                    ws.clone()
                ))
            } =>  {
                chars.into_iter().collect::<String>()
            })
            .add_error_info(capture!(
                bind_span!('"', quote) => Box::new(move |err: &mut FurthestFailError|{
                err.add_extra_label(quote,"unmatched quote",ariadne::Color::Blue);
            }) as Box<dyn Fn(&mut FurthestFailError)>
            )),
        );

        let array = capture!({
            commit_on((ws.clone(), '['),
            (
                ws.clone(),
                optional((
                    bind!(element.clone(), *elements),
                    many((
                        ','.try_insert_if_missing("missing comma"), ws.clone(),
                        many((unwanted(',', "missing element"), ws.clone())),
                        bind!(element.clone(), *elements),
                        negative_lookahead(':')
                    ))
                )), ws.clone(), many((unwanted(',', "trailing comma"), ws.clone())),
                ']'.try_insert_if_missing("missing closing ']'"), ws.clone()
            ))
        } =>  {
            JsonValue::Array(elements)
        });

        let key_value_pair = Rc::new(
            capture!({
                (
                bind!(raw_string.clone(), key), ':', ws.clone(),
                bind!(element.clone(), value),
                )
            } => {
                (key, value)
            })
            .with_label("key-value pair"),
        );

        let object = capture!({
                commit_on((ws.clone(), '{'),
                (
                ws.clone(),
                optional((
                    bind!(key_value_pair.clone(), *key_value_pairs),
                    many(
                        commit_on(
                            ',', (
                                ws.clone(),
                                bind!(key_value_pair.clone(), *key_value_pairs),
                            )
                        )
                    ),
                    many((unwanted(',', "trailing comma"), ws.clone()))
                )),
                '}'.try_insert_if_missing("missing closing '}'"), ws.clone()
                )
                )
        } => {
            JsonValue::Object(key_value_pairs)
        })
        .with_label("object");

        let string = capture!( bind!(raw_string.clone(), s)  => JsonValue::String(s));

        // let invalid_element = capture!(
        //     (
        //         unwanted(one_or_more(

        //         ))
        //     )
        // )
        capture!((
            ws.clone(), 
            bind!(one_of((
                object, 
                array, 
                string, 
                number, 
                boolean, 
                null
            )), result), 
            ws.clone()
        ) => result)
        .with_label("element")
    })
}

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "json".to_string());
    let Some(path) = args.next() else {
        eprintln!("Usage: {program} <path-to-json-file>");
        process::exit(2);
    };

    if args.next().is_some() {
        eprintln!("Usage: {program} <path-to-json-file>");
        process::exit(2);
    }

    let sample = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read '{path}': {err}");
            process::exit(1);
        }
    };

    let parser = get_json_grammar();
    match marser::parse(parser, sample.as_str()) {
        Ok((value, errors)) => {
            // eprintln!("--- Ariadne ---");
            // ParserError::eprint_many(&errors, path.as_str(), sample.as_str());
            ParserError::eprint_many_miette(&errors, path.as_str(), sample.as_str());
            // eprintln!("--- annotate-snippets ---");
            // ParserError::eprint_many_annotate_snippets(&errors, path.as_str(), sample.as_str());
            println!("\n--- Recovered JSON: ---");
            println!("{}", value.serialize_pretty());
        }
        Err(err) => err.eprint_ariadne(path.as_str(), sample.as_str()),
    }
}
