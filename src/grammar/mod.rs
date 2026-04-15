pub mod capture;
pub mod context;
pub mod error_handler;
pub mod label;
pub mod matcher;
pub mod parser;
// pub mod span;
use crate::Capture;
use crate::grammar::capture::bind_result;
use crate::grammar::{
    context::ParserContext,
    error_handler::{EmptyErrorHandler, ParserError},
    matcher::{
        any_token::AnyToken, commit_matcher::commit_on, negative_lookahead::negative_lookahead,
    },
    parser::Parser,
};
use std::rc::Rc;

pub fn parse<Pars>(parser: Pars, src: &str) -> Result<(Pars::Output, Vec<ParserError>), ParserError>
where
    Pars: Parser<char>,
{
    let tokens: Vec<char> = src.chars().collect();
    let mut error_handler = EmptyErrorHandler;
    let mut context = ParserContext::new(&tokens);
    let mut pos = 0;
    let parser = Rc::new(parser);

    let parser = Capture::<((::std::option::Option<_>,), (), ()), _, _>::new(
        |(result,), (), ()| {
            {
                commit_on(
                    (),
                    (
                        bind_result(parser.clone(), result),
                        negative_lookahead(AnyToken),
                    ),
                )
            }
        },
        |(result,), (), ()| result,
    );
    let result = parser
        .parse(&mut context, &mut error_handler, &mut pos)?
        .unwrap();
    Ok((result, context.get_errors()))
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use macros::capture;

    use crate::grammar::{
        capture::{Capture, bind_result, bind_span},
        matcher::{
            ToMatcher, multiple::many, one_of::one_of, one_or_more::one_or_more,
            optional::optional, parser_matcher::ParserMatcher,
        },
        parser::{
            ParserObjSafe, range_parser::RangeParser, single_token::SingleTokenParser,
            token_parser::TokenParser,
        },
    };

    use super::*;

    #[test]
    fn test_capture_macro() {
        let letter_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_alphabetic(),
            |token: &char| token.to_string(),
        ));
        let word_parser = Rc::new(capture!(
            {
                (
                    bind!(letter_parser.clone(),
                *letters),
                    many(bind!(letter_parser.clone(), *letters)),
                )
            } => {
                letters.into_iter().collect::<String>()
            }
        ));
        (1..3).contains(&2);
        let digit_parser = TokenParser::new(
            |token: &char| token.is_digit(10),
            |token: &char| token.to_string(),
        );

        let number_parser = capture!(
            {
                (
                    bind!(&digit_parser, *digits),//,
                    many(bind!(&digit_parser, *digits)),
                )
            } => {
                println!("Captured digits: {:?}", digits);
                digits.into_iter().collect::<String>()
            }
        );

        let identifier_parser = capture!(
        {
            (
                bind!(one_of((RangeParser::new('a'..='z'), RangeParser::new('A'..='Z'))), *tokens),
                many(bind!(
                    one_of((
                        RangeParser::new('a'..='z'),
                        RangeParser::new('A'..='Z'),
                        RangeParser::new('0'..='9'),
                        SingleTokenParser::new('_')
                    )),
                    *tokens
                )),
            )
        }   => {
                    tokens.into_iter().collect::<String>()
                }
            );

        // assert_eq!(
        //     number_parser.parse(Rc::new(ParserContext::new(vec!['1', '2', '3'])), &mut 0),
        //     Ok("123".to_string())
        // // );
        // assert!(check(&number_parser, "123"));
        // assert_eq!(parse(&number_parser, "123"), Ok("123".to_string()));
        // println!("{}", parse(&number_parser, "123abc").unwrap());
        // println!("{}", parse(&identifier_parser, "var_name123").unwrap());

        let func_parser = Rc::new(Capture::<
            (
                (
                    ::std::option::Option<_>,
                    ::std::option::Option<(usize, usize)>,
                    ::std::option::Option<(usize, usize)>,
                ),
                (::std::vec::Vec<_>, ::std::vec::Vec<(usize, usize)>),
                (::std::option::Option<_>,),
            ),
            _,
            _,
        >::new(
            |(name, fn_keyword_span, name_span), (params, param_spans), (body,)| {
                {
                    (
                        bind_span(
                            ParserMatcher::new(&identifier_parser, "fn".to_string()),
                            fn_keyword_span.clone(),
                        ),
                        one_or_more(" ".to_matcher()),
                        bind_span(
                            bind_result(word_parser.clone(), name.clone()),
                            name_span.clone(),
                        ),
                        many(" ".to_matcher()),
                        "(".to_matcher(),
                        many(" ".to_matcher()),
                        bind_span(
                            bind_result(word_parser.clone(), params.clone()),
                            param_spans.clone(),
                        ),
                        many((
                            many(" ".to_matcher()),
                            ",".to_matcher(),
                            many(" ".to_matcher()),
                            bind_span(
                                bind_result(word_parser.clone(), params.clone()),
                                param_spans.clone(),
                            ),
                        )),
                        many(" ".to_matcher()),
                        ")".to_matcher(),
                        many(" ".to_matcher()),
                        optional(bind_result(word_parser.clone(), body.clone())),
                    )
                }
            },
            |(name, fn_keyword_span, name_span), (params, param_spans), (body,)| {
                println!(
                    "Captured function: name={}, params={:?}, body={:?}",
                    name, params, body
                );
                println!(
                    "Spans: fn_keyword_span={:?}, name_span={:?}, param_spans={:?}",
                    fn_keyword_span, name_span, param_spans
                );
                format!(
                    "Function: name={}, params={:?}, body={:?}",
                    name, params, body
                )
            },
        ));
        let func_parser: Box<dyn ParserObjSafe<char, Output = String>> = Box::new(func_parser);
        // let func_parser = Rc::new(Capture::<
        //     (
        //         (
        //             ::std::option::Option<_>,
        //             ::std::option::Option<span::Span>,
        //             ::std::option::Option<span::Span>,
        //         ),
        //         (::std::vec::Vec<_>, ::std::vec::Vec<span::Span>),
        //         (::std::option::Option<_>,),
        //     ),
        //     _,
        //     _,
        // >::new(
        //     |(name, fn_keyword_span, name_span), (params, param_spans), (body,)| {
        //         {
        //             seq((
        //                 bind_span(
        //                     ParserMatcher::new(&identifier_parser, "fn".to_string()),
        //                     fn_keyword_span,
        //                 ),
        //                 one_or_more(" ".to_matcher()),
        //                 bind_span(bind_result(word_parser.clone(), name.clone()), name_span),
        //                 many(" ".to_matcher()),
        //                 "(".to_matcher(),
        //                 many(" ".to_matcher()),
        //                 bind_span(
        //                     bind_result(word_parser.clone(), params.clone()),
        //                     param_spans.clone(),
        //                 ),
        //                 many(seq((
        //                     many(" ".to_matcher()),
        //                     ",".to_matcher(),
        //                     many(" ".to_matcher()),
        //                     bind_span(
        //                         bind_result(word_parser.clone(), params.clone()),
        //                         param_spans.clone(),
        //                     ),
        //                 ))),
        //                 many(" ".to_matcher()),
        //                 ")".to_matcher(),
        //                 many(" ".to_matcher()),
        //                 optional(bind_result(word_parser.clone(), body.clone())),
        //             ))
        //         }
        //     },
        //     |(name, fn_keyword_span, name_span), (params, param_spans), (body,)| {
        //         {
        //             // Here we can use the captured values and spans to construct a more detailed output
        //         }
        //     },
        // ));
        // assert_eq!(
        //     func_parser.parse(
        //         Rc::new(ParserContext::new(vec![
        //             'f', 'n', ' ', 'm', 'a', 'i', 'n', '(', 'x', ',', ' ', 'y', ')', ' ', '{', '}'
        //         ])),
        //         &mut 0
        //     ),
        //     Ok("Function: name=main, params=[x, y], body=None".to_string())
        // );
        // assert_eq!(
        //     parse(func_parser.clone(), "fn main(x, y)"),
        //     Ok("Function: name=main, params=[x, y], body=None".to_string())
        // );

        // parse(&func_parser, "fn main(x, y)").unwrap();
    }
}
