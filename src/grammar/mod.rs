pub mod capture;
pub mod context;
pub mod error_handler;
pub mod label;
pub mod matcher;
pub mod parser;
use std::{
    ops::Deref,
    sync::atomic::{self, AtomicUsize},
};

use crate::grammar::context::ParserContext;

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
pub trait IsCheckable<T> {
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
}
// impl IsCheckable for all types that deref to an IsCheckable
impl<Token, T, C> IsCheckable<Token> for T
where
    T: Deref<Target = C>,
    C: IsCheckable<Token>,
{
    fn calc_check(&self, context: &ParserContext<Token>, pos: &mut usize) -> bool {
        (**self).calc_check(context, pos)
    }
}

pub trait Grammar<T> {
    fn check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
    fn check_no_advance(&self, context: &ParserContext<T>, pos: &usize) -> bool {
        let mut pos = *pos;
        self.check(context, &mut pos)
    }
}

// impl Grammar for all Parsers / Matchers, using memoization to optimize repeated checks
impl<T, G> Grammar<T> for G
where
    G: HasId + IsCheckable<T> + ?Sized,
{
    fn check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        let id = self.id();
        if let Some(&result) = context.memo_table.borrow().get(&(id, *pos)) {
            if let Some(new_pos) = result {
                *pos = new_pos; // Update the position to the memoized value
            }
            return result.is_some();
        }
        let old_pos = *pos;
        let result = self.calc_check(context, pos);
        context
            .memo_table
            .borrow_mut()
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
        let letter_parser = Rc::new(TokenParser::<char, String, _, _>::new(
            |token: &char| token.is_alphabetic(),
            |token: &char| token.to_string(),
        ));
        let word_parser = Rc::new(capture!(
            {
                seq((
                    bind!(letter_parser.clone(),
                *letters),
                    many(bind!(letter_parser.clone(), *letters)),
                ))
            } => {
                letters.into_iter().collect::<String>()
            }
        ));
        let digit_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_digit(10),
            |token: &char| token.to_string(),
        ));

        let number_parser = capture!(
            {
                seq((
                    bind!(digit_parser.clone(), *digits),
                    many(bind!(digit_parser.clone(), *digits)),
                ))
            } => {
                digits.into_iter().collect::<String>()
            }
        );

        assert_eq!(
            number_parser.parse(Rc::new(ParserContext::new(vec!['1', '2', '3'])), &mut 0),
            Ok("123".to_string())
        );

        let func_parser = capture!(
            {
                seq((
                    "fn".to_matcher(),
                    one_or_more(" ".to_matcher()),
                    bind!(word_parser.clone(), name),
                    many(" ".to_matcher()),
                    "(".to_matcher(),
                    many(" ".to_matcher()),
                    bind!(word_parser.clone(), *params),
                    many(seq((
                        many(" ".to_matcher()),
                        ",".to_matcher(),
                        many(" ".to_matcher()),
                        bind!(word_parser.clone(), *params),
                    ))),
                    many(" ".to_matcher()),
                    ")".to_matcher(),
                    many(" ".to_matcher()),
                    optional(
                        bind!(word_parser.clone(), ?body)
                    ),
                ))
            } => {
                format!(
                    "Function: name={}, params=[{}], body={}",
                    name,
                    params.into_iter().map(|p| p).collect::<Vec<_>>().join(", "),
                    body.map_or("None".to_string(), |b| b)
                )
            }
        );
        assert_eq!(
            func_parser.parse(
                Rc::new(ParserContext::new(vec![
                    'f', 'n', ' ', 'm', 'a', 'i', 'n', '(', 'x', ',', ' ', 'y', ')', ' ', '{', '}'
                ])),
                &mut 0
            ),
            Ok("Function: name=main, params=[x, y], body=None".to_string())
        );
    }
}
