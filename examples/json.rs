use std::{collections::HashMap, rc::Rc};

use marser_macros::capture;

use marser::{
    error::FurthestFailError, label::WithLabel, matcher::{
        Matcher, commit_matcher::commit_on, multiple::many, 
        one_or_more::one_or_more, optional::optional,
    }, one_of::one_of, parser::{Parser, deferred::recursive, token_parser::TokenParser}
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

pub fn get_json_grammar() -> impl Parser<char, Output = JsonValue> {
    recursive(|element| {
        let element = Rc::new(element.with_label("element"));
        let ws = Rc::new(many(one_of((' ', '\t', '\n', '\r'))));

        let null = capture!(("null", ws.clone())  => JsonValue::Null );
        let bool_false = capture!(("false", ws.clone())  => JsonValue::Boolean(false) );
        let bool_true = capture!(("true", ws.clone())  => JsonValue::Boolean(true) );
        let boolean = one_of((bool_true, bool_false));

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

fn main() {
    let parser = get_json_grammar();
    let sample = r#"{"hello": ["world", 42, true]}"#;
    match marser::parse(parser, sample) {
        Ok((value, warnings)) => {
            println!("Parsed value: {value:?}");
            if !warnings.is_empty() {
                println!("Warnings: {}", warnings.len());
            }
        }
        Err(err) => err.eprint_ariadne("example.json", sample),
    }
}
