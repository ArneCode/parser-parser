pub mod capture;
pub mod context;
pub mod matcher;
pub mod parser;
use std::{
    array,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::atomic::{self, AtomicUsize},
    usize,
};

use crate::grammar::{
    capture::{Capture, capture_property},
    context::{MatcherContext, ParserContext},
    matcher::Matcher,
    parser::Parser,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
fn get_next_id() -> usize {
    NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed)
}
pub trait HasId {
    fn id(&self) -> usize;
}
pub trait AstNode {}
pub trait Token {}

pub trait IsCheckable<T: Token> {
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
}
// impl IsCheckable for all Rc<IsCheckable>
impl<T, C> IsCheckable<T> for Rc<C>
where
    T: Token,
    C: IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        (**self).calc_check(context, pos)
    }
}

// impl HasId for all Rc<HasId>
impl<H> HasId for Rc<H>
where
    H: HasId,
{
    fn id(&self) -> usize {
        (**self).id()
    }
}

trait Grammar<T: Token> {
    fn check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
    fn check_no_advance(&self, context: &ParserContext<T>, pos: &usize) -> bool {
        let mut pos = *pos;
        self.check(context, &mut pos)
    }
}

// impl Grammar for all Parsers / Matchers, using memoization to optimize repeated checks
impl<T: Token, G: HasId + IsCheckable<T> + ?Sized> Grammar<T> for G {
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

trait MyAstNode: AstNode {}

struct Node {
    value: String,
}
impl AstNode for Node {}
impl MyAstNode for Node {}
#[macro_export]
macro_rules! bind {
    ($($tokens:tt)*) => {
        compile_error!("The `bind!` macro can only be used inside a `capture!` block.");
    };
}
// fn boxed<T: Token, N: AstNode + ?Sized, M: Matcher<T, Output = N> + 'static>(
//     m: M,
// ) -> Box<dyn Matcher<T, Output = N>> {
//     Box::new(m)
// }
#[cfg(test)]
mod tests {
    use macros::capture;

    use crate::grammar::{
        matcher::{ToMatcher, multiple::many, sequence::seq},
        parser::token_parser::TokenParser,
    };

    use super::*;

    #[test]
    fn test_capture_macro() {
        let word_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_alphabetic(),
            |token: &char| {
                Box::new(Node {
                    value: token.to_string(),
                })
            },
        ));
        let digit_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_digit(10),
            |token: &char| {
                Box::new(Node {
                    value: token.to_string(),
                })
            },
        ));

        let number_parser = capture!(
            {
                seq((
                    bind!(digit_parser.clone(), *digits),
                    many(bind!(digit_parser.clone(), *digits)),
                ))
            } => {
                // In scope:
                //   digits: Vec<Box<N>>
                Box::new(Node {
                    value: digits.into_iter().map(|d| d.value).collect(),
                })
            }
        );

        let func_parser = capture!(
            {
                seq((
                    "fn".to_matcher(),
                    bind!(word_parser.clone(), name),
                    "(".to_matcher(),
                    bind!(word_parser.clone(), *params),
                    many(seq((
                        ",".to_matcher(),
                        bind!(word_parser.clone(), *params),
                    ))),
                    ")".to_matcher(),
                    bind!(word_parser.clone(), ?body),
                ))
            } => {
                // In scope:
                //   name:   Box<N>
                //   params: Vec<Box<N>>
                //   body:   Option<Box<N>>
                Box::new(Node {
                    value: format!(
                        "Function: name={}, params=[{}], body={}",
                        name.value,
                        params.into_iter().map(|p| p.value).collect::<Vec<_>>().join(", "),
                        body.map_or("None".to_string(), |b| b.value)
                    ),
                })
            }
        );

        assert_eq!(
            number_parser
                .parse(Rc::new(ParserContext::new(vec!['1', '2', '3'])), &mut 0)
                .unwrap()
                .value,
            "123"
        );

        assert_eq!(
            func_parser
                .parse(
                    Rc::new(ParserContext::new(vec![
                        'f', 'n', ' ', 'x', ' ', '(', 'y', ',', 'z', ')', ' ', 'b', 'o', 'd', 'y'
                    ])),
                    &mut 0
                )
                .unwrap()
                .value,
            "Function: name=x, params=[y, z], body=body"
        );

        /*
                What this should expand to:
                let func_parser = Capture::new::<1, 1, 1, _>(
            // grammar_factory: property arrays destructured into named bindings
            |[name]:   [SingleProperty;   1],
             [params]: [MultipleProperty; 1],
             [body]:   [OptionalProperty; 1]| {
                //                  ↓ `as name`   replaced     ↓ `as *params` replaced (×2, Copy)
                Sequence::new(vec![
                    StringMatcher::new("fn"),
                    CaptureProperty::new(word_parser, name),
                    StringMatcher::new("("),
                    CaptureProperty::new(expression_parser, params),
                    many(
                        Sequence::new(vec![
                            StringMatcher::new(","),
                            CaptureProperty::new(expression_parser, params), // params copied
                        ])
                    ),
                    StringMatcher::new(")"),
                    CaptureProperty::new(block_parser, body),
                    //                             ↑ `as ?body` replaced
                ])
            },
            // constructor: extract → run user block
            |mut __ctx| {
                let name   = __ctx.match_result.single_matches[0]
                    .take()
                    .expect("capture!: single capture `name` was never set\n...");
                let params = ::std::mem::take(&mut __ctx.match_result.multiple_matches[0]);
                let body   = __ctx.match_result.optional_matches[0].take();

                Box::new(FuncDefNode::new(name, params, body))
            },
        );
                 */
    }
}
