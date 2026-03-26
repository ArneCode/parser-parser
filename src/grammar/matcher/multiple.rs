use std::{marker::PhantomData, ops::Deref};

use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::Matcher,
};

pub struct Multiple<T, MContext, Match> {
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, MContext)>,
}

impl<T, MContext, Match> Multiple<T, MContext, Match>
where
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

// pub fn many<T, N, Match>(matcher: Match) -> Multiple<T, N, Match>
// where
//     T: Token + 'static,
//     N: AstNode + ?Sized + 'static,
//     Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
// {
//     Multiple::new(matcher)
// }
pub fn many<T, MContext, Match>(matcher: Match) -> Multiple<T, MContext, Match>
where
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    Multiple::new(matcher)
}
// impl<T, N, Match> Matcher<T> for Multiple<T, N, Match>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
// {
//     type Output = N;

//     fn match_pattern(
//         &self,
//         context: &mut MatcherContext<T, Self::Output>,
//         pos: &mut usize,
//     ) -> Result<(), String> {
//         while self.matcher.check_no_advance(context, pos) {
//             self.matcher.match_pattern(context, pos)?;
//         }
//         Ok(())
//     }
// }
impl<T, MContext, Match> Matcher<T, MContext> for Multiple<T, MContext, Match>
where
    MContext: Deref<Target = ParserContext<T>>,
    Match: Matcher<T, MContext> + HasId + IsCheckable<T>,
{
    fn match_pattern(&self, context: &mut MContext, pos: &mut usize) -> Result<(), String> {
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

// impl<T, N, Match> IsCheckable<T> for Multiple<T, N, Match>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
// {
//     fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
//         // advance pos
//         while self.matcher.check(context, pos) {}
//         return true;
//     }
// }
impl<T, MContext, Match> IsCheckable<T> for Multiple<T, MContext, Match>
where
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        while self.matcher.check(context, pos) {}
        true
    }
}

// impl<T, N, Match> HasId for Multiple<T, N, Match>
// where
//     T: Token,
//     N: AstNode + ?Sized,
//     Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
// {
//     fn id(&self) -> usize {
//         self.id
//     }
// }

impl<T, MContext, Match> HasId for Multiple<T, MContext, Match> {
    fn id(&self) -> usize {
        self.id
    }
}
