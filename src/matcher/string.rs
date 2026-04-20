//! Literal string / `&str` / [`char`] matchers over a `char` token stream.

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::MatchRunner,
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

impl<InpFam, MRes> super::internal::MatcherImpl<InpFam, MRes> for StringMatcher
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = char>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        for expected in &self.expected {
            if input.next() != Some(*expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn maybe_label_internal(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.expected.iter().collect::<String>()))
    }
}

impl<InpFam, MRes> super::internal::MatcherImpl<InpFam, MRes> for &str
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = char>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        for expected in self.chars() {
            if input.next() != Some(expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn maybe_label_internal(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.to_string()))
    }
}

// impl for char
impl<InpFam, MRes> super::internal::MatcherImpl<InpFam, MRes> for char
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = char>,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        if input.next() == Some(*self) {
            return Ok(true);
        }
        Ok(false)
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
