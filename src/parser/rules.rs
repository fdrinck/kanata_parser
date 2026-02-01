use super::{ParseError, ParseErrorKind, Parser};
use crate::{Command, DepKind, LogKind, RetireKind, StrRef};
use memchr::memchr2;
use std::convert::TryFrom;

impl<'a> Parser<'a> {
    fn spaces(&mut self) {
        while let Some(b' ' | b'\t') = self.current() {
            self.bump();
        }
    }

    fn lineend(&mut self) {
        if let Some(b'\r' | b'\n') = self.current() {
            self.bump();
            if let Some(b'\n') = self.current() {
                self.bump();
            }
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), ParseError> {
        if let Some(actual) = self.current()
            && actual == expected
        {
            self.bump();
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::UnexpectedCharacter))
        }
    }

    fn eat(&mut self, expected: u8) -> bool {
        if let Some(actual) = self.current()
            && actual == expected
        {
            self.bump();
            true
        } else {
            false
        }
    }

    fn tab(&mut self) -> Result<(), ParseError> {
        self.expect(b'\t')
    }

    fn single_digit(&mut self) -> Result<u8, ParseError> {
        if let Some(actual) = self.current()
            && actual.is_ascii_digit()
        {
            self.bump();
            Ok(actual)
        } else {
            Err(self.error(ParseErrorKind::ExpectedValue))
        }
    }

    fn parse_u64(&mut self) -> Result<u64, ParseError> {
        let mut v = 0u64;
        let r = self.rest();
        let mut i = 0;
        while i < r.len() && r[i].is_ascii_digit() {
            v = v * 10 + (r[i] - b'0') as u64;
            i += 1;
        }
        if i > 0 {
            self.advance(i);
            Ok(v)
        } else {
            Err(self.error(ParseErrorKind::ExpectedValue))
        }
    }

    fn parse_i32(&mut self) -> Result<i32, ParseError> {
        if let Some(c) = self.current() {
            let mut neg = false;
            if c == b'-' {
                neg = true;
                self.bump();
            } else if c == b'+' {
                self.bump();
            }
            let num = i32::try_from(self.parse_u64()?)
                .map_err(|_| self.error(ParseErrorKind::ValueTooBig))?;
            if neg { Ok(-num) } else { Ok(num) }
        } else {
            Err(self.error(ParseErrorKind::UnexpectedEof))
        }
    }

    fn parse_u32(&mut self) -> Result<u32, ParseError> {
        let v = self.parse_u64()?;
        u32::try_from(v).map_err(|_| self.error(ParseErrorKind::ValueTooBig))
    }

    fn text(&mut self) -> Result<StrRef, ParseError> {
        let start = self.get_offset();
        let rest = self.rest();

        let len = match memchr2(b'\r', b'\n', rest) {
            Some(i) => i,
            None => rest.len(),
        };

        if len == 0 {
            return Err(self.error(ParseErrorKind::ExpectedText));
        }

        self.advance(len);
        Ok(StrRef::new(start as u64, len as u16))
    }

    pub(super) fn parse_header(&mut self) -> Result<Command, ParseError> {
        let kanata = b"Kanata\t";
        if !self.rest().starts_with(kanata) {
            return Err(self.error(ParseErrorKind::InvalidHeader));
        }
        self.advance(kanata.len());
        let version = self.parse_u32()?; // version
        self.spaces();
        self.lineend();
        Ok(Command::Kanata { version })
    }

    pub(super) fn parse_c(&mut self) -> Result<Command, ParseError> {
        self.bump(); // C
        let abs = self.eat(b'=');
        self.tab()?;
        let value = self.parse_i32()?;
        self.spaces();
        self.lineend();
        Ok(Command::Cycle { abs, value })
    }

    pub(super) fn parse_i(&mut self) -> Result<Command, ParseError> {
        self.bump(); // I
        self.tab()?;
        let id_file = self.parse_u32()?;
        self.tab()?;
        let id_sim = self.parse_u32()?;
        self.tab()?;
        let thread = self.parse_u32()?;
        self.spaces();
        self.lineend();
        Ok(Command::Instruction {
            id_in_file: id_file,
            id_in_sim: id_sim,
            thread_id: thread,
        })
    }

    pub(super) fn parse_l(&mut self) -> Result<Command, ParseError> {
        self.bump(); // L
        self.tab()?;
        let id = self.parse_u32()?;
        self.tab()?;
        let kind = LogKind::try_from(self.single_digit()?).map_err(|e| self.error(e))?;
        self.tab()?;
        let text = self.text()?;
        self.lineend();
        Ok(Command::Log { id, kind, text })
    }

    pub(super) fn parse_pipeline(&mut self, start: bool) -> Result<Command, ParseError> {
        self.bump(); // S or E
        self.tab()?;
        let id = self.parse_u32()?;
        self.tab()?;
        let lane = self.parse_u32()?;
        self.tab()?;
        let name = self.text()?;
        self.lineend();
        Ok(Command::Pipeline {
            start,
            id,
            lane_id: lane,
            name,
        })
    }

    pub(super) fn parse_r(&mut self) -> Result<Command, ParseError> {
        self.bump(); // R
        self.tab()?;
        let id = self.parse_u32()?;
        self.tab()?;
        let retire = self.parse_u32()?;
        self.tab()?;
        let kind = RetireKind::try_from(self.single_digit()?).map_err(|e| self.error(e))?;
        self.spaces();
        self.lineend();
        Ok(Command::Retire { id, retire, kind })
    }

    pub(super) fn parse_w(&mut self) -> Result<Command, ParseError> {
        self.bump(); // W
        self.tab()?;
        let c = self.parse_u32()?;
        self.tab()?;
        let p = self.parse_u32()?;
        self.tab()?;
        let kind = DepKind::try_from(self.single_digit()?).map_err(|e| self.error(e))?;
        self.spaces();
        self.lineend();
        Ok(Command::Dep {
            consumer_id: c,
            producer_id: p,
            kind,
        })
    }
}
