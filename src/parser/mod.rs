use crate::Command;

#[derive(Debug)]
pub enum ParseErrorKind {
    InvalidHeader,
    InvalidLogKind,
    InvalidRetireKind,
    InvalidDepKind,
    TextTooLong,
    ExpectedValue,
    ValueTooBig,
    ExpectedText,
    UnexpectedCharacter,
    UnexpectedEof,
}

#[derive(Debug)]
pub struct ParseError {
    pub offset: usize,
    pub kind: ParseErrorKind,
}

mod primitive;
pub use primitive::Parser;
mod rules;

impl<'a> Iterator for Parser<'a> {
    type Item = (usize, Result<Command, ParseError>);

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.get_offset();
        if let Some(b) = self.current() {
            let res = match b {
                b'K' => self.parse_header(),
                b'C' => self.parse_c(),
                b'I' => self.parse_i(),
                b'L' => self.parse_l(),
                b'S' => self.parse_pipeline(true),
                b'E' => self.parse_pipeline(false),
                b'R' => self.parse_r(),
                b'W' => self.parse_w(),
                _ => Err(self.error(ParseErrorKind::UnexpectedCharacter)),
            };
            Some((offset, res))
        } else {
            None
        }
    }
}
