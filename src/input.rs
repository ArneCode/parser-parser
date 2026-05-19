//! Input abstractions for parsers and matchers.
//!
//! `marser` works over any source that can yield tokens one-by-one via [`Input`].
//! Built-in implementations cover `&str` (token = `char`) and `&[T]`
//! (token = `&T`). [`SliceableInput`] adds the ability to recover the exact
//! consumed slice, which powers helpers like `bind_slice!`.
//!
//! For `&str` inputs, [`Input::try_consume_prefix_bytes`] offers a memcmp-style
//! fast path for ASCII literal prefixes on UTF-8 byte storage.

use std::ops::Range;

/// Stream-like source that parsers and matchers can read from.
pub trait Input<'src> {
    /// Token type yielded by this input.
    type Token: 'src;
    /// Position type used to mark offsets and spans.
    type Pos: Clone + Into<usize> + 'src;
    /// Starting position for a fresh parse.
    fn start_pos(&self) -> Self::Pos;
    /// Read one token at `pos`, advancing it on success.
    fn read_token(&mut self, pos: &mut Self::Pos) -> Option<Self::Token>;

    /// If supported, compare the byte range at `pos` to `prefix` using raw bytes
    /// and advance `pos` by `prefix.len()` on success.
    ///
    /// - `None`: this input type does not implement the fast path (callers should
    ///   fall back to per-token matching).
    /// - `Some(true)` / `Some(false)`: prefix matched / did not match; on `true`,
    ///   `pos` was advanced.
    ///
    /// For `&str`, this is only used when every byte in `prefix` is ASCII and `pos`
    /// is a UTF-8 code point boundary (the usual parser invariant).
    fn try_consume_prefix_bytes(&mut self, pos: &mut Self::Pos, prefix: &[u8]) -> Option<bool> {
        let _ = (pos, prefix);
        None
    }
}

/// [`Input`] that can also return a slice for a consumed range.
pub trait SliceableInput<'src>: Input<'src> {
    /// Borrowed slice type returned for matched ranges.
    type Slice: 'src;
    /// Return the input slice covering `range`.
    fn slice(&self, range: Range<Self::Pos>) -> Self::Slice;
}

impl<'src> Input<'src> for &'src str {
    type Token = char;
    type Pos = usize;

    #[inline]
    fn start_pos(&self) -> Self::Pos {
        0
    }

    #[inline]
    fn read_token(&mut self, pos: &mut Self::Pos) -> Option<Self::Token> {
        let bytes = self.as_bytes();
        let i = *pos;
        let b = *bytes.get(i)?;
        // ASCII fast path: avoids a `chars()` iterator and extra slicing on every token (large
        // win for JSON and other mostly-ASCII grammars). `*pos` is always a UTF-8 code point start.
        if b.is_ascii() {
            *pos += 1;
            return Some(char::from(b));
        }
        let ch = self.get(i..)?.chars().next()?;
        *pos += ch.len_utf8();
        Some(ch)
    }

    #[inline]
    fn try_consume_prefix_bytes(&mut self, pos: &mut Self::Pos, prefix: &[u8]) -> Option<bool> {
        if prefix.is_empty() {
            return Some(true);
        }
        if !prefix.iter().all(|b| b.is_ascii()) {
            return None;
        }
        let i = *pos;
        let bytes = self.as_bytes();
        if bytes.get(i..i + prefix.len())? == prefix {
            *pos += prefix.len();
            Some(true)
        } else {
            Some(false)
        }
    }
}

impl<'src> SliceableInput<'src> for &'src str {
    type Slice = &'src str;

    fn slice(&self, range: Range<Self::Pos>) -> Self::Slice {
        &self[range.start..range.end]
    }
}

impl<'src, T> Input<'src> for &'src [T] {
    type Token = &'src T;
    type Pos = usize;

    #[inline]
    fn start_pos(&self) -> Self::Pos {
        0
    }

    #[inline]
    fn read_token(&mut self, pos: &mut Self::Pos) -> Option<Self::Token> {
        if *pos < self.len() {
            let token = &self[*pos];
            *pos += 1;
            Some(token)
        } else {
            None
        }
    }
}

impl<'src, T> SliceableInput<'src> for &'src [T] {
    type Slice = &'src [T];

    fn slice(&self, range: Range<Self::Pos>) -> Self::Slice {
        &self[range.start..range.end]
    }
}

// a struct that wraps an Input and keeps track of the current position
pub(crate) struct InputStream<'src, I: Input<'src>> {
    input: I,
    pos: I::Pos,
}

impl<'src, I: Input<'src>> InputStream<'src, I> {
    #[inline]
    pub(crate) fn new(input: I) -> Self {
        let pos = input.start_pos();
        Self { input, pos }
    }

    #[inline]
    pub(crate) fn next(&mut self) -> Option<I::Token> {
        self.input.read_token(&mut self.pos)
    }

    #[inline]
    pub(crate) fn get_pos(&self) -> I::Pos {
        self.pos.clone()
    }

    #[inline]
    pub(crate) fn set_pos(&mut self, pos: I::Pos) {
        self.pos = pos;
    }

    /// Forwards to [`Input::try_consume_prefix_bytes`] on the underlying input.
    #[inline]
    pub(crate) fn try_consume_prefix_bytes(&mut self, prefix: &[u8]) -> Option<bool> {
        self.input.try_consume_prefix_bytes(&mut self.pos, prefix)
    }

    #[inline]
    pub(crate) fn slice(&self, range: Range<I::Pos>) -> I::Slice
    where
        I: SliceableInput<'src>,
    {
        self.input.slice(range)
    }
}

impl<'src> InputStream<'src, &'src [u8]> {
    /// Compares the current position against `prefix` in one step (memcmp).
    #[inline]
    pub(crate) fn try_consume_byte_prefix(&mut self, prefix: &[u8]) -> bool {
        if prefix.is_empty() {
            return true;
        }
        let i = self.pos;
        let buf = self.input;
        if buf.get(i..i + prefix.len()) == Some(prefix) {
            self.pos = i + prefix.len();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, InputStream};

    #[test]
    fn str_try_consume_prefix_advances_pos() {
        let mut stream = InputStream::new("hello");
        assert_eq!(stream.try_consume_prefix_bytes(b"hel"), Some(true));
        assert_eq!(stream.get_pos(), 3);
        assert_eq!(stream.try_consume_prefix_bytes(b"lo"), Some(true));
        assert_eq!(stream.get_pos(), 5);
    }

    #[test]
    fn str_try_consume_prefix_mismatch_returns_false() {
        let mut stream = InputStream::new("hello");
        assert_eq!(stream.try_consume_prefix_bytes(b"hi"), Some(false));
        assert_eq!(stream.get_pos(), 0);
    }

    #[test]
    fn str_try_consume_empty_prefix_is_true() {
        let mut stream = InputStream::new("x");
        assert_eq!(stream.try_consume_prefix_bytes(b""), Some(true));
        assert_eq!(stream.get_pos(), 0);
    }

    #[test]
    fn byte_stream_try_consume_byte_prefix() {
        let buf: &[u8] = b"nulltail";
        let mut stream = InputStream::new(buf);
        assert!(stream.try_consume_byte_prefix(b"null"));
        assert_eq!(stream.get_pos(), 4);
        assert!(!stream.try_consume_byte_prefix(b"nomatch"));
        assert_eq!(stream.get_pos(), 4);
    }

    #[test]
    fn str_try_consume_non_ascii_prefix_returns_none() {
        let mut stream = InputStream::new("x");
        assert_eq!(stream.try_consume_prefix_bytes(&[0xC3, 0xA9]), None);
        assert_eq!(stream.get_pos(), 0);
    }

    #[test]
    fn str_input_try_consume_via_trait() {
        let mut s: &str = "abc";
        let mut pos = 0usize;
        assert_eq!(
            Input::try_consume_prefix_bytes(&mut s, &mut pos, b"ab"),
            Some(true)
        );
        assert_eq!(pos, 2);
    }
}
