use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher, ToMatcher},
};

pub struct StringMatcher {
    expected: Vec<char>,
}

impl StringMatcher {
    fn new(expected: String) -> Self {
        Self {
            expected: expected.chars().collect(),
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

impl<'a, 'ctx, Runner> Matcher<Runner> for StringMatcher
where
    Runner: MatchRunner<'a, 'ctx, Token = char>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner(
        &self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, ParserError> {
        let context = runner.get_parser_context();
        let end_pos = *pos + self.expected.len();
        if end_pos > context.tokens.len() {
            *pos = context.tokens.len(); // Move to end if not enough tokens
            return Ok(false);
        }
        let slice = &context.tokens[*pos..end_pos];
        if slice == self.expected {
            *pos = end_pos; // Advance position
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// impl MaybeLabel<String> for StringMatcher {
//     fn maybe_label(&self) -> Option<String> {
//         None
//     }
// }
