//! Literal string / `&str` / [`char`] matchers over a `char` token stream.

use crate::{
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatchRunner, MatcherCombinator},
};

/// Matches a fixed run of characters (by Unicode scalar values).
#[derive(Clone, Debug)]
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

impl super::MatcherCombinator for StringMatcher {}

impl<'src, Inp: Input<'src, Token = char>, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for StringMatcher
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let ascii_only = self.expected.iter().all(|c| c.is_ascii());
        if ascii_only {
            let mut buf = vec![0u8; self.expected.len()];
            for (i, c) in self.expected.iter().enumerate() {
                buf[i] = *c as u8;
            }
            if let Some(ok) = input.try_consume_prefix_bytes(&buf) {
                return Ok(ok);
            }
        }
        for expected in &self.expected {
            if input.next() != Some(*expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(self.expected.iter().collect::<String>()))
    }
}

impl super::MatcherCombinator for &str {}

impl<'src, Inp: Input<'src, Token = char>, MRes> super::internal::MatcherImpl<'src, Inp, MRes>
    for &str
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if self.is_ascii() {
            if let Some(ok) = input.try_consume_prefix_bytes(self.as_bytes()) {
                return Ok(ok);
            }
            for &expected in self.as_bytes() {
                if input.next() != Some(expected as char) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        for expected in self.chars() {
            if input.next() != Some(expected) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[inline]
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

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        if input.next() == Some(*self) {
            return Ok(true);
        }
        Ok(false)
    }
    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(*self))
    }
}

impl MatcherCombinator for char {}

// --- Byte slice / `u8` matchers for `Inp = &'src [u8]` (`Token = &'src u8`) ---

impl MatcherCombinator for &[u8] {}

impl<'src, MRes> super::internal::MatcherImpl<'src, &'src [u8], MRes> for &[u8] {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, &'src [u8], MRes = MRes>,
        'src: 'a,
    {
        Ok(input.try_consume_byte_prefix(self))
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(
            std::str::from_utf8(self).unwrap_or("<bytes>").to_string(),
        ))
    }
}

impl MatcherCombinator for u8 {}

impl<'src, MRes> super::internal::MatcherImpl<'src, &'src [u8], MRes> for u8 {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = true;

    #[inline]
    fn match_with_runner<'a, Runner, M: crate::mode::Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, &'src [u8]>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, &'src [u8], MRes = MRes>,
        'src: 'a,
    {
        if let Some(tok) = input.next()
            && *tok == *self
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        Some(Box::new(format!("byte {self}")))
    }
}

// impl MaybeLabel<String> for StringMatcher {
//     fn maybe_label(&self) -> Option<String> {
//         None
//     }
// }
