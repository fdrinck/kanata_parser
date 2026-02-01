use super::{ParseError, ParseErrorKind};

pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}
impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub(super) fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    pub(super) fn get_offset(&self) -> usize {
        self.pos
    }

    pub(super) fn rest(&self) -> &'a [u8] {
        &self.input[self.get_offset()..]
    }

    pub(super) fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            offset: self.get_offset(),
            kind,
        }
    }

    pub(super) fn bump(&mut self) {
        self.advance(1);
    }

    pub(super) fn current(&mut self) -> Option<u8> {
        self.rest().first().copied()
    }
}
