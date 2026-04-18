use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::MatchRunner,
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

impl<MRes> super::internal::MatcherImpl<char, MRes> for StringMatcher {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = char, MRes = MRes>,
        'ctx: 'a,
    {
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

    fn maybe_label_internal(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.expected.iter().collect::<String>()))
    }
}

impl<MRes> super::internal::MatcherImpl<char, MRes> for &str {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = char, MRes = MRes>,
        'ctx: 'a,
    {
        let context = runner.get_parser_context();
        let expected_len = self.chars().count();
        let end_pos = *pos + expected_len;
        if end_pos > context.tokens.len() {
            *pos = context.tokens.len();
            return Ok(false);
        }
        let slice = &context.tokens[*pos..end_pos];
        if slice.iter().copied().eq(self.chars()) {
            *pos = end_pos;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn maybe_label_internal(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.to_string()))
    }
}

// impl for char
impl<MRes> super::internal::MatcherImpl<char, MRes> for char {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = char, MRes = MRes>,
        'ctx: 'a,
    {
        let context = runner.get_parser_context();

        if *pos < context.tokens.len() {
            *pos += 1; // Advance position
            if context.tokens[*pos - 1] == *self {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    fn maybe_label_internal(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(*self))
    }
}

// impl MaybeLabel<String> for StringMatcher {
//     fn maybe_label(&self) -> Option<String> {
//         None
//     }
// }
