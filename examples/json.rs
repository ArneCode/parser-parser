use std::{env, fs, process, rc::Rc};

use marser_macros::capture;

use marser::{
    error::{FurthestFailError, ParserError},
    label::{WithLabel, WithTrace},
    matcher::{
        AnyToken, MatcherCombinator, commit_matcher::commit_on, if_error::{if_error, if_error_else_fail}, multiple::many,
        negative_lookahead, one_or_more::one_or_more, optional::optional, positive_lookahead,
        unwanted::unwanted,
    },
    one_of::one_of,
    parser::{Parser, ParserCombinator, deferred::recursive, token_parser::TokenParser},
};
#[cfg(feature = "parser-trace")]
use marser::trace::load::TraceFormat;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue<'src> {
    Invalid(&'src str),
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue<'src>>),
    Object(Vec<(String, JsonValue<'src>)>),
}

impl<'src> JsonValue<'src> {
    pub fn serialize(&self) -> String {
        match self {
            Self::Invalid(slice) => format!("invalid('{slice}')"),
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
            Self::Invalid(slice) => format!("invalid('{slice}')"),
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

pub fn get_json_grammar<'src>() -> impl Parser<'src, &'src str, Output = JsonValue<'src>> {
    recursive(|element| {
        let ws = Rc::new(
            many(one_of((' ', '\t', '\n', '\r')))
                .with_label("whitespace"),
        );

        let null = capture!(("null", ws.clone()) => JsonValue::Null).with_label("null");
        let bool_false =
            capture!(("false", ws.clone()) => JsonValue::Boolean(false)).with_label("false");
        let bool_true =
            capture!(("true", ws.clone()) => JsonValue::Boolean(true)).with_label("true");
        let boolean = one_of((bool_true, bool_false)).with_label("boolean");
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
            // many(one_of(('+', '-', '0'..='9', '.', 'e', 'E'))),
            // JsonValue::Invalid,
            capture!(
                bind_slice!(many(one_of(('+', '-', '0'..='9', '.', 'e', 'E'))), slice) => JsonValue::Invalid(slice)
            ),
        )
        .with_label("number");

        let character = Rc::new(
            TokenParser::new(
                |c| *c != '"' && *c != '\\' && (*c as u32) >= 0x20,
                |x| *x,
            )
            .with_label("string character"),
        );
        let hex_digit = Rc::new(
            one_of(('0'..='9', 'a'..='f', 'A'..='F')).with_label("hex digit"),
        );
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
        })
        .with_label("escape sequence");
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
        })
        .with_label("unicode escape");
        let raw_string = Rc::new(
            capture!({
                commit_on(
                    '"',(
                    many(one_of((
                        bind!(character.clone(), *chars),
                        bind!(escaped_char, *chars),
                        bind!(unicode_escape, *chars),
                    ))),
                    '"'.try_insert_if_missing("missing closing quote"),
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
        )
        .with_label("quoted string")
        .maybe_erase_types();

        let array = capture!({
            commit_on((ws.clone(), '['),
            (
                ws.clone().trace(),
                optional((
                    bind!(element.clone(), *elements).trace(),
                    many((
                        ','.trace().try_insert_if_missing("missing comma"),
                        ws.clone().trace(),
                        if_error(many((unwanted(',', "missing element"), ws.clone())))
                            .trace(),
                        bind!(element.clone(), *elements).trace(),
                        if_error(negative_lookahead(':')).trace()
                    ))
                    .trace()
                )).trace(),
                ws.clone().trace(),
                if_error(many((unwanted(',', "trailing comma"), ws.clone())))
                    .trace(),
                ']'.try_insert_if_missing("missing closing ']'"),
                ws.clone().trace()
            ))
        } =>  {
            JsonValue::Array(elements)
        })
        .with_label("array")
        .maybe_erase_types();

        let key_value_pair = Rc::new(
            capture!({
                (
                bind!(raw_string.clone(), key).trace(),
                ':',
                ws.clone().trace(),
                bind!(element.clone(), value).trace(),
                )
            } => {
                (key, value)
            })
            .with_label("key-value pair"),
        ).maybe_erase_types();

        let object = capture!({
                commit_on((ws.clone(), '{'),
                (
                ws.clone().trace(),
                optional((
                    bind!(key_value_pair.clone(), *key_value_pairs),
                    many((
                        ','.trace().try_insert_if_missing("missing comma"),
                        ws.clone().trace(),
                        bind!(key_value_pair.clone(), *key_value_pairs),
                    )),
                    if_error(
                        many((unwanted(',', "trailing comma"), ws.clone()))
                            .trace(),
                    )
                    .trace()
                )),
                '}'.try_insert_if_missing("missing closing '}'"),
                ws.clone().trace()
                )
                )
        } => {
            JsonValue::Object(key_value_pairs)
        })
        .with_label("object")
        .maybe_erase_types();

        let string = raw_string
            .map_output(JsonValue::String)
            .with_label("string");

        let invalid_element = capture!(
            if_error_else_fail(bind_slice!(unwanted(one_or_more(
                (
                    negative_lookahead(one_of((
                        '{',
                        '[',
                        '"',
                        '-',
                        '0'..='9',
                        ',',
                        ']',
                        '}',
                        ':',
                        one_of((' ', '\t', '\n', '\r'))
                    ))),
                    AnyToken
                )
            ), "invalid element"), slice),
            ) => JsonValue::Invalid(slice)
        )
        .with_label("invalid element")
        .maybe_erase_types();
        capture!((
            ws.clone().trace(),
            bind!(one_of((
                object.trace(),
                array.trace(),
                string.trace(),
                number.trace(),
                boolean.trace(),
                null.trace(),
                invalid_element.trace()
            )), result),
            ws.clone().trace()
        ) => result)
        .with_label("element")
    })
}
fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "json".to_string());
    let mut path = "tests/data/json1.json".to_string();
    #[cfg(feature = "parser-trace")]
    let mut trace_mode: Option<&'static str> = None;
    #[cfg(feature = "parser-trace")]
    let mut trace_file: Option<String> = None;
    #[cfg(feature = "parser-trace")]
    let mut trace_format = TraceFormat::Jsonl;

    #[cfg(feature = "parser-trace")]
    let mut pending_trace_file = false;
    #[cfg(feature = "parser-trace")]
    let mut pending_trace_format = false;
    for arg in args {
        #[cfg(feature = "parser-trace")]
        if pending_trace_file {
            trace_file = Some(arg);
            pending_trace_file = false;
            continue;
        }
        #[cfg(feature = "parser-trace")]
        if pending_trace_format {
            trace_format = match arg.as_str() {
                "json" => TraceFormat::Json,
                "jsonl" => TraceFormat::Jsonl,
                _ => {
                    eprintln!("Invalid --trace-format value '{arg}'. Use json or jsonl.");
                    process::exit(2);
                }
            };
            pending_trace_format = false;
            continue;
        }
        #[cfg(feature = "parser-trace")]
        if arg == "--trace-jsonl" {
            trace_mode = Some("jsonl");
            continue;
        }
        #[cfg(feature = "parser-trace")]
        if arg == "--trace-text" {
            trace_mode = Some("text");
            continue;
        }
        #[cfg(feature = "parser-trace")]
        if arg == "--trace-file" {
            pending_trace_file = true;
            continue;
        }
        #[cfg(feature = "parser-trace")]
        if arg == "--trace-format" {
            pending_trace_format = true;
            continue;
        }

        if path != "tests/data/json1.json" {
            #[cfg(feature = "parser-trace")]
            eprintln!("Usage: {program} <path-to-json-file> [--trace-text|--trace-jsonl] [--trace-file <path>] [--trace-format json|jsonl]");
            #[cfg(not(feature = "parser-trace"))]
            eprintln!("Usage: {program} <path-to-json-file>");
            process::exit(2);
        }
        path = arg;
    }

    let sample = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read '{path}': {err}");
            process::exit(1);
        }
    };

    let parser = get_json_grammar();
    #[cfg(feature = "parser-trace")]
    if let Some(trace_file_path) = trace_file {
        match marser::parse_with_trace_to_file(parser, sample.as_str(), &trace_file_path, trace_format) {
            Ok((value, errors)) => {
                ParserError::eprint_many_miette(&errors, path.as_str(), sample.as_str());
                eprintln!("trace written to {trace_file_path}");
                println!("\n--- Recovered JSON: ---");
                println!("{}", value.serialize_pretty());
            }
            Err(marser::ParseWithTraceToFileError::Parse(err)) => err.eprint_ariadne(path.as_str(), sample.as_str()),
            Err(marser::ParseWithTraceToFileError::Io(err)) => {
                eprintln!("Failed to write trace file '{trace_file_path}': {err}");
                process::exit(1);
            }
        }
    } else {
        match marser::parse_with_trace(parser, sample.as_str()) {
            Ok((value, errors, trace)) => {
            // eprintln!("--- Ariadne ---");
            // ParserError::eprint_many(&errors, path.as_str(), sample.as_str());
            ParserError::eprint_many_miette(&errors, path.as_str(), sample.as_str());
            if trace_mode == Some("jsonl") {
                let mut sink = std::io::stderr();
                let _ = trace.write_jsonl(&mut sink);
            } else if trace_mode == Some("text") {
                eprintln!("{}", trace.to_timeline());
            }
            // eprintln!("--- annotate-snippets ---");
            // ParserError::eprint_many_annotate_snippets(&errors, path.as_str(), sample.as_str());
            println!("\n--- Recovered JSON: ---");
            println!("{}", value.serialize_pretty());
        }
        Err(err) => err.eprint_ariadne(path.as_str(), sample.as_str()),
    }
    }

    #[cfg(not(feature = "parser-trace"))]
    match marser::parse(parser, sample.as_str()) {
        Ok((value, errors)) => {
            ParserError::eprint_many_miette(&errors, path.as_str(), sample.as_str());
            println!("\n--- Recovered JSON: ---");
            println!("{}", value.serialize_pretty());
        }
        Err(err) => err.eprint_ariadne(path.as_str(), sample.as_str()),
    }
}
