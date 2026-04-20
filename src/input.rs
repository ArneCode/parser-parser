use std::ops::Range;

pub trait Input<'src> {
    type Token: 'src;
    type Pos: Clone + Into<usize> + 'src;
    fn start_pos(&self) -> Self::Pos;
    fn read_token(&mut self, pos: &mut Self::Pos) -> Option<Self::Token>;
}

pub trait InputFamily {
    type In<'src>: Input<'src>;
}

impl InputFamily for str {
    type In<'src> = &'src str;
}

impl<T: 'static> InputFamily for [T] {
    type In<'src> = &'src [T];
}

pub trait SliceableInput<'src>: Input<'src> {
    type Slice: 'src;
    fn slice(&self, range: Range<&Self::Pos>) -> Self::Slice;
}

impl<'src> Input<'src> for &'src str {
    type Token = char;
    type Pos = usize;

    fn start_pos(&self) -> Self::Pos {
        0
    }

    fn read_token(&mut self, pos: &mut Self::Pos) -> Option<Self::Token> {
        let mut chars = self[*pos..].chars();
        let token = chars.next()?;
        *pos += token.len_utf8();
        Some(token)
    }
}

impl<'src> SliceableInput<'src> for &'src str {
    type Slice = &'src str;

    fn slice(&self, range: Range<&Self::Pos>) -> Self::Slice {
        &self[*range.start..*range.end]
    }
}

impl<'src, T> Input<'src> for &'src [T] {
    type Token = &'src T;
    type Pos = usize;

    fn start_pos(&self) -> Self::Pos {
        0
    }

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

    fn slice(&self, range: Range<&Self::Pos>) -> Self::Slice {
        &self[*range.start..*range.end]
    }
}

// a struct that wraps an Input and keeps track of the current position
pub(crate) struct InputStream<'src, I: Input<'src>> {
    input: I,
    pos: I::Pos,
}

impl<'src, I: Input<'src>> InputStream<'src, I> {
    pub(crate) fn new(input: I) -> Self {
        let pos = input.start_pos();
        Self { input, pos }
    }

    pub(crate) fn next(&mut self) -> Option<I::Token> {
        self.input.read_token(&mut self.pos)
    }

    pub(crate) fn get_pos(&self) -> I::Pos {
        self.pos.clone()
    }

    pub(crate) fn set_pos(&mut self, pos: I::Pos) {
        self.pos = pos;
    }

    pub(crate) fn slice(&self, range: Range<&I::Pos>) -> Option<I::Slice>
    where
        I: SliceableInput<'src>,
    {
        Some(self.input.slice(range))
    }
}
