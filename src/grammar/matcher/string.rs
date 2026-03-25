use std::marker::PhantomData;

use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::{Matcher, ToMatcher},
};

pub struct StringMatcher<N: AstNode + ?Sized> {
    expected: String,
    id: usize,
    _phantom: PhantomData<N>,
}

impl<N: AstNode + ?Sized> StringMatcher<N> {
    fn new(expected: String) -> Self {
        Self {
            expected,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

// impl ToMatcher<char, N> for String {
impl<N: AstNode + ?Sized + 'static> ToMatcher<char, N> for String {
    type MatcherType = StringMatcher<N>;

    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.clone())
    }
}

impl<N: AstNode + ?Sized + 'static> ToMatcher<char, N> for &str {
    type MatcherType = StringMatcher<N>;
    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.to_string())
    }
}

impl<N: AstNode + ?Sized> HasId for StringMatcher<N> {
    fn id(&self) -> usize {
        self.id
    }
}
impl Token for char {}
impl<N: AstNode + ?Sized> IsCheckable<char> for StringMatcher<N> {
    fn calc_check(&self, context: &ParserContext<char>, pos: &mut usize) -> bool {
        let end_pos = *pos + self.expected.len();
        if end_pos > context.tokens.len() {
            return false;
        }
        let slice: String = context.tokens[*pos..end_pos].iter().collect();
        if slice == self.expected {
            *pos = end_pos; // Advance position on success
            true
        } else {
            false
        }
    }
}

impl<N: AstNode + ?Sized> Matcher<char> for StringMatcher<N> {
    type Output = N;
    fn match_pattern(
        &self,
        _context: &mut MatcherContext<char, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.check(_context, pos) {
            Ok(())
        } else {
            Err(format!("Expected '{}' at position {}", self.expected, pos))
        }
    }
}
