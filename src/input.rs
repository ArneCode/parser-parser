//! Input abstractions for parsers and matchers.
//!
//! `marser` works over any source that can yield tokens one-by-one via [`Input`].
//! Built-in implementations cover `&str` (token = `char`) and `&[T]`
//! (token = `&T`). [`SliceableInput`] adds the ability to recover the exact
//! consumed slice, which powers helpers like `bind_slice!`.

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
        let mut chars = self[*pos..].chars();
        let token = chars.next()?;
        *pos += token.len_utf8();
        Some(token)
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

    #[inline]
    pub(crate) fn slice(&self, range: Range<I::Pos>) -> I::Slice
    where
        I: SliceableInput<'src>,
    {
        self.input.slice(range)
    }
}
