use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{MatcherContext, ParserContext},
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
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
impl ToMatcher for String {
    type MatcherType = StringMatcher;

    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.clone())
    }
}

impl ToMatcher for &str {
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
    fn calc_check(
        &self,
        context: &mut ParserContext<char, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        let end_pos = *pos + self.expected.len();
        if end_pos > context.tokens.len() {
            *pos = context.tokens.len(); // Move to end if not enough tokens
            return false;
        }
        let slice: String = context.tokens[*pos..end_pos].iter().collect();
        *pos = end_pos; // Advance position
        slice == self.expected
    }
}

impl<MRes> Matcher<char, MRes> for StringMatcher {
    fn match_pattern(
        &self,
        context: &mut MatcherContext<char, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.check(context.parser_context, pos) {
            Ok(())
        } else {
            Err(format!("Expected '{}' at position {}", self.expected, pos))
        }
    }
}

impl MaybeLabel<String> for StringMatcher {
    fn maybe_label(&self) -> Option<String> {
        Some(self.expected.clone())
    }
}
