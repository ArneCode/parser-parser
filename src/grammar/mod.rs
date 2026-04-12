pub mod capture;
pub mod context;
pub mod error_handler;
pub mod label;
pub mod matcher;
pub mod parser;
pub mod span;
use std::sync::atomic::AtomicUsize;

use crate::grammar::error_handler::ErrorHandler;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
// fn get_next_id() -> usize {
//     NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed)
// }
// pub trait HasId {
//     fn id(&self) -> usize;
// }
// // impl HasId for all types that deref to a HasId
// impl<T, H> HasId for T
// where
//     T: Deref<Target = H>,
//     H: HasId,
// {
//     fn id(&self) -> usize {
//         (**self).id()
//     }
// }
// pub trait IsCheckable<Token> {
//     fn calc_check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool;
// }
// // impl IsCheckable for all types that deref to an IsCheckable
// impl<Inner, Outer, Token> IsCheckable<Token> for Outer
// where
//     Outer: Deref<Target = Inner>,
//     Inner: IsCheckable<Token>,
// {
//     fn calc_check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool {
//         (**self).calc_check(context, pos)
//     }
// }

// pub trait Grammar<Token> {
//     fn check(&self, context: &mut ParserContext<Token, impl ErrorHandler>, pos: &mut usize)
//     -> bool;
//     fn check_no_advance(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &usize,
//     ) -> bool {
//         let mut pos = *pos;
//         self.check(context, &mut pos)
//     }
// }

// // impl Grammar for all Parsers / Matchers, using memoization to optimize repeated checks
// impl<G, Token> Grammar<Token> for G
// where
//     G: HasId + IsCheckable<Token> + MaybeLabel<String> + ?Sized,
// {
//     fn check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool {
//         let id = self.id();
//         if let Some(&result) = context.memo_table.get(&(id, *pos)) {
//             if let Some(new_pos) = result {
//                 *pos = new_pos; // Update the position to the memoized value
//             }
//             return result.is_some();
//         }
//         let error_idx = context.error_handler.register_start();
//         let old_pos = *pos;
//         let result = self.calc_check(context, pos);
//         context
//             .memo_table
//             .insert((id, old_pos), if result { Some(*pos) } else { None });
//         if !result {
//             context.error_handler.register_error(
//                 &self,
//                 error_idx,
//                 context.match_start,
//                 (old_pos, *pos),
//             );
//             *pos = old_pos; // Reset position on failure
//         }
//         result
//     }
// }

// #[macro_export]
// macro_rules! bind {
//     ($($tokens:tt)*) => {
//         compile_error!("The `bind!` macro can only be used inside a `capture!` block.");
//     };
// }

// pub fn parse<Pars>(parser: Pars, src: &str) -> Result<Pars::Output, String>
// where
//     Pars: Parser<char> + Grammar<char>,
// {
//     let mut tokens: Vec<char> = src.chars().collect();
//     let mut error_handler = EmptyErrorHandler::default();
//     let mut context = ParserContext::new(&mut tokens, &mut error_handler);
//     let mut pos = 0;
//     if parser.check(&mut context, &mut pos) {
//         pos = 0; // Reset position for actual parsing
//         parser.parse(&mut context, &mut pos)
//     } else {
//         pos = 0; // Reset position for error reporting
//         let mut error_handler = MultiErrorHandler::default();
//         let mut context = ParserContext::new(&mut tokens, &mut error_handler);
//         parser.check(&mut context, &mut pos);
//         error_handler.render_report("call to parse function", src);
//         Err(format!("Parsing failed at position {}", pos))
//     }
// }
// pub fn parse<'a, Pars>(parser: Pars, src: &str) -> Result<Pars::Output, String>
// where
//     Pars: Parser<'a, char>,
// {
//     let mut tokens: Vec<char> = src.chars().collect();
//     let mut error_handler = EmptyErrorHandler::default();
//     let mut context = ParserContext::new(&mut tokens);
//     let mut pos = 0;
//     if let Some(result) = parser.parse(&mut context, &mut error_handler, &mut pos)? {
//         if pos == tokens.len() {
//             Ok(result)
//         } else {
//             Err(format!(
//                 "Parsing failed: Unconsumed input at position {}",
//                 pos
//             ))
//         }
//     } else {
//         let mut error_handler = MultiErrorHandler::default();
//         let mut context = ParserContext::new(&mut tokens);
//         parser.parse(&mut context, &mut pos);
//         error_handler.render_report("call to parse function", src);
//         Err(format!("Parsing failed at position {}", pos))
//     }
//     // if parser.check(&mut context, &mut pos) {
//     //     pos = 0; // Reset position for actual parsing
//     //     parser.parse(&mut context, &mut pos)
//     // } else {
//     //     pos = 0; // Reset position for error reporting
//     //     let mut error_handler = MultiErrorHandler::default();
//     //     let mut context = ParserContext::new(&mut tokens, &mut error_handler);
//     //     parser.check(&mut context, &mut pos);
//     //     error_handler.render_report("call to parse function", src);
//     //     Err(format!("Parsing failed at position {}", pos))
//     // }
// }
// pub fn check<Pars>(parser: Pars, src: &str) -> bool
// where
//     Pars: Grammar<char>,
// {
//     let mut tokens: Vec<char> = src.chars().collect();
//     let mut error_handler = EmptyErrorHandler::default();
//     let mut context = ParserContext::new(&mut tokens, &mut error_handler);
//     let mut pos = 0;
//     parser.check(&mut context, &mut pos) && pos == tokens.len()
// }

#[cfg(test)]
mod tests {
    use std::{ops::RangeBounds, rc::Rc, vec};

    use macros::capture;

    use crate::grammar::{
        capture::{Capture, bind_result, bind_span},
        matcher::{
            ToMatcher, multiple::many, one_or_more::one_or_more, optional::optional,
            parser_matcher::ParserMatcher,
        },
        parser::{
            Parser, one_of::OneOfParser, range_parser::RangeParser,
            single_token::SingleTokenParser, token_parser::TokenParser,
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
                bind!(OneOfParser::new((RangeParser::new('a'..='z'), RangeParser::new('A'..='Z'))), *tokens),
                many(bind!(
                    OneOfParser::new((
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
                    ::std::option::Option<span::Span>,
                    ::std::option::Option<span::Span>,
                ),
                (::std::vec::Vec<_>, ::std::vec::Vec<span::Span>),
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
