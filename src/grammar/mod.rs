pub mod capture;
pub mod context;
pub mod error_handler;
pub mod label;
pub mod matcher;
pub mod parser;
use std::{
    ops::{Deref, Mul},
    sync::atomic::{self, AtomicUsize},
};

use crate::grammar::{
    context::ParserContext,
    error_handler::{EmptyErrorHandler, ErrorHandler, MultiErrorHandler},
    parser::Parser,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
fn get_next_id() -> usize {
    NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed)
}
pub trait HasId {
    fn id(&self) -> usize;
}
// impl HasId for all types that deref to a HasId
impl<T, H> HasId for T
where
    T: Deref<Target = H>,
    H: HasId,
{
    fn id(&self) -> usize {
        (**self).id()
    }
}
pub trait IsCheckable<Token> {
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool;
}
// impl IsCheckable for all types that deref to an IsCheckable
impl<Inner, Outer, Token> IsCheckable<Token> for Outer
where
    Outer: Deref<Target = Inner>,
    Inner: IsCheckable<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        (**self).calc_check(context, pos)
    }
}

pub trait Grammar<Token> {
    fn check(&self, context: &mut ParserContext<Token, impl ErrorHandler>, pos: &mut usize)
    -> bool;
    fn check_no_advance(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &usize,
    ) -> bool {
        let mut pos = *pos;
        self.check(context, &mut pos)
    }
}

// impl Grammar for all Parsers / Matchers, using memoization to optimize repeated checks
impl<G, Token> Grammar<Token> for G
where
    G: HasId + IsCheckable<Token> + ?Sized,
{
    fn check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        let id = self.id();
        if let Some(&result) = context.memo_table.get(&(id, *pos)) {
            if let Some(new_pos) = result {
                *pos = new_pos; // Update the position to the memoized value
            }
            return result.is_some();
        }
        let old_pos = *pos;
        let result = self.calc_check(context, pos);
        context
            .memo_table
            .insert((id, old_pos), if result { Some(*pos) } else { None });
        if !result {
            *pos = old_pos; // Reset position on failure
        }
        result
    }
}

#[macro_export]
macro_rules! bind {
    ($($tokens:tt)*) => {
        compile_error!("The `bind!` macro can only be used inside a `capture!` block.");
    };
}

fn parse<Pars>(parser: Pars, src: &str) -> Result<Pars::Output, String>
where
    Pars: Parser<char> + Grammar<char>,
{
    let mut tokens: Vec<char> = src.chars().collect();
    let mut error_handler = EmptyErrorHandler::default();
    let mut context = ParserContext::new(&mut tokens, &mut error_handler);
    let mut pos = 0;
    if parser.check(&mut context, &mut pos) {
        parser.parse(&mut context, &mut pos)
    } else {
        let mut error_handler = MultiErrorHandler::default();
        let mut context = ParserContext::new(&mut tokens, &mut error_handler);
        parser.check(&mut context, &mut pos);
        error_handler.render_report("call to parse function", src);
        Err(format!("Parsing failed at position {}", pos))
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, vec};

    use macros::capture;

    use crate::grammar::{
        capture::{Capture, capture_property},
        matcher::{
            ToMatcher, multiple::many, one_or_more::one_or_more, optional::optional, sequence::seq,
        },
        parser::{Parser, token_parser::TokenParser},
    };

    use super::*;

    #[test]
    fn test_capture_macro() {
        let letter_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_alphabetic(),
            |token: &char| token.to_string(),
        ));
        // let word_parser = Rc::new(capture!(
        //     {
        //         seq((
        //             bind!(letter_parser.clone(),
        //         *letters),
        //             many(bind!(letter_parser.clone(), *letters)),
        //         ))
        //     } => {
        //         letters.into_iter().collect::<String>()
        //     }
        // ));
        let digit_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_digit(10),
            |token: &char| token.to_string(),
        ));

        let number_parser = Capture::<((), (::std::vec::Vec<String>,), ()), _, _>::new(
            |(), (digits,), ()| {
                {
                    seq::<_, ((), (Vec<String>,), ())>((
                        capture_property::<_, _, _, _, (), (Vec<_>,), ()>(
                            digit_parser.clone(),
                            digits.clone(),
                        ),
                        many::<((), (Vec<String>,), ()), _>(capture_property::<
                            _,
                            _,
                            _,
                            _,
                            (),
                            (Vec<_>,),
                            (),
                        >(
                            digit_parser.clone(), digits.clone()
                        )),
                    ))
                }
            },
            |(), (digits,), ()| digits.into_iter().collect::<String>(),
        );

        // assert_eq!(
        //     number_parser.parse(Rc::new(ParserContext::new(vec!['1', '2', '3'])), &mut 0),
        //     Ok("123".to_string())
        // );

        // let func_parser = capture!(
        //     {
        //         seq((
        //             "fn".to_matcher(),
        //             one_or_more(" ".to_matcher()),
        //             bind!(word_parser.clone(), name),
        //             many(" ".to_matcher()),
        //             "(".to_matcher(),
        //             many(" ".to_matcher()),
        //             bind!(word_parser.clone(), *params),
        //             many(seq((
        //                 many(" ".to_matcher()),
        //                 ",".to_matcher(),
        //                 many(" ".to_matcher()),
        //                 bind!(word_parser.clone(), *params),
        //             ))),
        //             many(" ".to_matcher()),
        //             ")".to_matcher(),
        //             many(" ".to_matcher()),
        //             optional(
        //                 bind!(word_parser.clone(), ?body)
        //             ),
        //         ))
        //     } => {
        //         format!(
        //             "Function: name={}, params=[{}], body={}",
        //             name,
        //             params.into_iter().map(|p| p).collect::<Vec<_>>().join(", "),
        //             body.map_or("None".to_string(), |b| b)
        //         )
        //     }
        // );
        // assert_eq!(
        //     func_parser.parse(
        //         Rc::new(ParserContext::new(vec![
        //             'f', 'n', ' ', 'm', 'a', 'i', 'n', '(', 'x', ',', ' ', 'y', ')', ' ', '{', '}'
        //         ])),
        //         &mut 0
        //     ),
        //     Ok("Function: name=main, params=[x, y], body=None".to_string())
        // );
    }
}
