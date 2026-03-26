use std::ops::Deref;

use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::ParserContext,
    get_next_id,
    matcher::{Matcher, ToMatcher},
};

pub struct StringMatcher {
    expected: String,
    id: usize,
}

impl StringMatcher {
    fn new(expected: String) -> Self {
        Self {
            expected,
            id: get_next_id(),
        }
    }
}

// impl ToMatcher<char, N> for String {
impl<N> ToMatcher<char, N> for String
where
    N: Deref<Target = ParserContext<char>>,
{
    type MatcherType = StringMatcher;

    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.clone())
    }
}

impl<N> ToMatcher<char, N> for &str
where
    N: Deref<Target = ParserContext<char>>,
{
    type MatcherType = StringMatcher;
    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.to_string())
    }
}

impl HasId for StringMatcher {
    fn id(&self) -> usize {
        self.id
    }
}
impl IsCheckable<char> for StringMatcher {
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

impl<N> Matcher<char, N> for StringMatcher
where
    N: Deref<Target = ParserContext<char>>,
{
    fn match_pattern(&self, context: &mut N, pos: &mut usize) -> Result<(), String> {
        if self.check(context, pos) {
            Ok(())
        } else {
            Err(format!("Expected '{}' at position {}", self.expected, pos))
        }
    }
}
