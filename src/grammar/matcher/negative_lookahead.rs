use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::Matcher,
};
use std::marker::PhantomData;
pub struct NegativeLookahead<T, Check> {
    checker: Check,
    id: usize,
    _phantom: PhantomData<T>,
}

impl<T, Check> NegativeLookahead<T, Check>
where
    Check: Grammar<T>,
{
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// !e  — negative lookahead. Succeeds without consuming if `e` would *not* match.
// pub fn negative_lookahead<T, N, Check>(checker: Check) -> NegativeLookahead<T, N, Check>
// where
//     T: Token + 'static,
//     N: AstNode + ?Sized + 'static,
//     Check: HasId + IsCheckable<T> + 'static,
// {
//     NegativeLookahead::new(checker)
// }
pub fn negative_lookahead<T, Check>(checker: Check) -> NegativeLookahead<T, Check>
where
    Check: Grammar<T>,
{
    NegativeLookahead::new(checker)
}

// impl<T, N, Check> HasId for NegativeLookahead<T, N, Check>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Check: HasId + IsCheckable<T>,
// {
//     fn id(&self) -> usize {
//         self.id
//     }
// }
impl<T, Check> HasId for NegativeLookahead<T, Check> {
    fn id(&self) -> usize {
        self.id
    }
}

// impl<T, N, Check> IsCheckable<T> for NegativeLookahead<T, N, Check>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Check: HasId + IsCheckable<T>,
// {
//     fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
//         // Peek — pos must not move.  Success means the inner check *failed*.
//         !self.checker.check_no_advance(context, pos)
//     }
// }
impl<T, Check> IsCheckable<T> for NegativeLookahead<T, Check>
where
    Check: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Peek — pos must not move.  Success means the inner check *failed*.
        !self.checker.check_no_advance(context, pos)
    }
}

// impl<T, N, Check> Matcher<T> for NegativeLookahead<T, N, Check>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Check: HasId + IsCheckable<T>,
// {
//     type Output = N;

//     fn match_pattern(
//         &self,
//         context: &mut MatcherContext<T, Self::Output>,
//         pos: &mut usize,
//     ) -> Result<(), String> {
//         if !self.checker.check_no_advance(context, pos) {
//             Ok(()) // pos unchanged, nothing captured
//         } else {
//             Err(format!(
//                 "negative lookahead failed: forbidden pattern matched at position {}",
//                 pos
//             ))
//         }
//     }
// }
impl<T, Check> Matcher<T, ParserContext<T>> for NegativeLookahead<T, Check>
where
    Check: Grammar<T>,
{
    fn match_pattern(&self, context: &mut ParserContext<T>, pos: &mut usize) -> Result<(), String> {
        if !self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!(
                "negative lookahead failed: forbidden pattern matched at position {}",
                pos
            ))
        }
    }
}
