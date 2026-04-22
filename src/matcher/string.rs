//! Literal string / `&str` / [`char`] matchers over a `char` token stream.

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, MatcherCombinator},
};

/// Matches a fixed run of characters (by Unicode scalar values).
pub struct StringMatcher {
    expected: Vec<char>,
}

impl StringMatcher {
    /// Converts `expected` to a `Vec<char>` for matching.
    pub fn new(expected: String) -> Self {
        Self {
            expected: expected.chars().collect(),
        }
    }
}

impl<'src, Inp: Input<'src, Token = char>, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for StringMatcher
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        for expected in &self.expected {
            if input.next() != Some(*expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.expected.iter().collect::<String>()))
    }
}

impl<'src, Inp: Input<'src, Token = char>, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for &str
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        for expected in self.chars() {
            if input.next() != Some(expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.to_string()))
    }
}

// impl for char
impl<'src, Inp: Input<'src, Token = char>, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for char
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if input.next() == Some(*self) {
            return Ok(true);
        }
        Ok(false)
    }
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(*self))
    }
}

impl MatcherCombinator for char{}

// impl MaybeLabel<String> for StringMatcher {
//     fn maybe_label(&self) -> Option<String> {
//         None
//     }
// }
