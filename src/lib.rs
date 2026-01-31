use memchr::memchr2;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum ParseErrorKind {
    InvalidHeader,
    InvalidLogKind,
    InvalidRetireKind,
    InvalidDepKind,
    ExpectedValue,
    ExpectedText,
    UnexpectedCharacter,
    UnexpectedEof,
}

#[derive(Debug)]
pub struct ParseError {
    pub offset: usize,
    pub kind: ParseErrorKind,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum LogKind {
    LeftPane = b'0',
    MouseOver = b'1',
    Other = b'2',
}

impl TryFrom<u8> for LogKind {
    type Error = ParseErrorKind;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(LogKind::LeftPane),
            b'1' => Ok(LogKind::MouseOver),
            b'2' => Ok(LogKind::Other),
            _ => Err(ParseErrorKind::InvalidLogKind),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum RetireKind {
    Retire = b'0',
    Flush = b'1',
}

impl TryFrom<u8> for RetireKind {
    type Error = ParseErrorKind;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(RetireKind::Retire),
            b'1' => Ok(RetireKind::Flush),
            _ => Err(ParseErrorKind::InvalidRetireKind),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DepKind {
    WakeUp = b'0',
}

impl TryFrom<u8> for DepKind {
    type Error = ParseErrorKind;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(DepKind::WakeUp),
            _ => Err(ParseErrorKind::InvalidDepKind),
        }
    }
}

#[derive(Copy, Clone)]
pub struct StrRef(u64);

impl StrRef {
    pub fn new(offset: u64, len: u16) -> Self {
        Self((offset << 16) | len as u64)
    }

    pub fn offset(self) -> u64 {
        self.0 >> 16
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(self) -> u16 {
        self.0 as u16
    }
}

pub enum Command {
    Kanata {
        version: u8,
    },
    Cycle {
        abs: bool,
        value: i32,
    },
    Instruction {
        id_in_file: u8,
        id_in_sim: u8,
        thread_id: u8,
    },
    Log {
        id: u8,
        kind: LogKind,
        text: StrRef,
    },
    Pipeline {
        start: bool,
        id: u8,
        lane_id: u8,
        name: StrRef,
    },
    Retire {
        id: u8,
        retire: u8,
        kind: RetireKind,
    },
    Dep {
        consumer_id: u8,
        producer_id: u8,
        kind: DepKind,
    },
}

pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &'a [u8] {
        &self.input[self.pos..]
    }

    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            offset: self.pos,
            kind,
        }
    }

    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    fn bump(&mut self) {
        self.advance(1);
    }

    fn current(&mut self) -> Option<u8> {
        self.rest().first().copied()
    }

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
            let num = self.parse_u64()? as i32;
            if neg { Ok(-num) } else { Ok(num) }
        } else {
            Err(self.error(ParseErrorKind::UnexpectedEof))
        }
    }

    fn parse_u8(&mut self) -> Result<u8, ParseError> {
        self.parse_u64().map(|v| v as u8)
    }

    fn text(&mut self) -> Result<StrRef, ParseError> {
        let start = self.pos;
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

    fn parse_header(&mut self) -> Result<Command, ParseError> {
        let kanata = b"Kanata\t";
        if !self.rest().starts_with(kanata) {
            return Err(self.error(ParseErrorKind::InvalidHeader));
        }
        self.advance(kanata.len());
        let version = self.parse_u8()?; // version
        self.spaces();
        self.lineend();
        Ok(Command::Kanata { version })
    }

    fn parse_c(&mut self) -> Result<Command, ParseError> {
        self.bump(); // C
        let abs = self.eat(b'=');
        self.tab()?;
        let value = self.parse_i32()?;
        self.spaces();
        self.lineend();
        Ok(Command::Cycle { abs, value })
    }

    fn parse_i(&mut self) -> Result<Command, ParseError> {
        self.bump(); // I
        self.tab()?;
        let id_file = self.parse_u8()?;
        self.tab()?;
        let id_sim = self.parse_u8()?;
        self.tab()?;
        let thread = self.parse_u8()?;
        self.spaces();
        self.lineend();
        Ok(Command::Instruction {
            id_in_file: id_file,
            id_in_sim: id_sim,
            thread_id: thread,
        })
    }

    fn parse_l(&mut self) -> Result<Command, ParseError> {
        self.bump(); // L
        self.tab()?;
        let id = self.parse_u8()?;
        self.tab()?;
        let kind = LogKind::try_from(self.single_digit()?).map_err(|e| self.error(e))?;
        self.tab()?;
        let text = self.text()?;
        self.lineend();
        Ok(Command::Log { id, kind, text })
    }

    fn parse_pipeline(&mut self, start: bool) -> Result<Command, ParseError> {
        self.bump(); // S or E
        self.tab()?;
        let id = self.parse_u8()?;
        self.tab()?;
        let lane = self.parse_u8()?;
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

    fn parse_r(&mut self) -> Result<Command, ParseError> {
        self.bump(); // R
        self.tab()?;
        let id = self.parse_u8()?;
        self.tab()?;
        let retire = self.parse_u8()?;
        self.tab()?;
        let kind = RetireKind::try_from(self.single_digit()?).map_err(|e| self.error(e))?;
        self.spaces();
        self.lineend();
        Ok(Command::Retire { id, retire, kind })
    }

    fn parse_w(&mut self) -> Result<Command, ParseError> {
        self.bump(); // W
        self.tab()?;
        let c = self.parse_u8()?;
        self.tab()?;
        let p = self.parse_u8()?;
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

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Command, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(b) = self.current() {
            let res = match b {
                b'K' => self.parse_header(),
                b'C' => self.parse_c(),
                b'I' => self.parse_i(),
                b'L' => self.parse_l(),
                b'S' => self.parse_pipeline(true),
                b'E' => self.parse_pipeline(false),
                b'R' => self.parse_r(),
                b'W' => self.parse_w(),
                b'\n' => {
                    self.bump();
                    continue;
                }
                _ => {
                    self.skip_line();
                    continue;
                }
            };
            return Some(res);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;
    use insta::assert_snapshot;
    use std::fmt::Write;

    struct PrettyPrinter<'a> {
        input: &'a [u8],
        out: String,
    }

    impl<'a> PrettyPrinter<'a> {
        pub fn new(input: &'a [u8]) -> Self {
            Self {
                input,
                out: String::with_capacity(1024),
            }
        }

        fn strref(&self, s: StrRef) -> &'a [u8] {
            let off = s.offset() as usize;
            let len = s.len() as usize;
            &self.input[off..off + len]
        }

        pub fn finish(self) -> String {
            self.out
        }
    }

    impl<'a> PrettyPrinter<'a> {
        pub fn print(&mut self, cmd: Command) {
            match cmd {
                Command::Kanata { version } => {
                    let _ = writeln!(self.out, "Kanata version={}", version);
                }

                Command::Cycle { abs, value } => {
                    if abs {
                        let _ = writeln!(self.out, "Cycle ={}", value);
                    } else {
                        let _ = writeln!(self.out, "Cycle +{}", value);
                    }
                }

                Command::Instruction {
                    id_in_file,
                    id_in_sim,
                    thread_id,
                } => {
                    let _ = writeln!(
                        self.out,
                        "Instr file={} sim={} thread={}",
                        id_in_file, id_in_sim, thread_id
                    );
                }

                Command::Log { id, kind, text } => {
                    let kind = match kind {
                        LogKind::LeftPane => "left",
                        LogKind::MouseOver => "hover",
                        LogKind::Other => "other",
                    };

                    let txt = self.strref(text);
                    let txt = String::from_utf8_lossy(txt);

                    let _ = writeln!(self.out, "Log id={} kind={} text=\"{}\"", id, kind, txt);
                }

                Command::Pipeline {
                    start,
                    id,
                    lane_id,
                    name,
                } => {
                    let name = String::from_utf8_lossy(self.strref(name));
                    let _ = writeln!(
                        self.out,
                        "{}Stage id={} lane={} name={}",
                        if start { "Start" } else { "End" },
                        id,
                        lane_id,
                        name
                    );
                }

                Command::Retire { id, retire, kind } => {
                    let kind = match kind {
                        RetireKind::Retire => "retire",
                        RetireKind::Flush => "flush",
                    };

                    let _ = writeln!(self.out, "Retire id={} rid={} kind={}", id, retire, kind);
                }

                Command::Dep {
                    consumer_id,
                    producer_id,
                    kind,
                } => {
                    let kind = match kind {
                        DepKind::WakeUp => "wakeup",
                    };

                    let _ = writeln!(
                        self.out,
                        "Dep {} <- {} ({})",
                        consumer_id, producer_id, kind
                    );
                }
            }
        }
    }

    fn parse_and_pretty_print(input: &[u8]) -> Result<String, ParseError> {
        let parser = Parser::new(input);
        let mut pp = PrettyPrinter::new(input);
        for cmd in parser {
            let cmd = cmd?;
            pp.print(cmd);
        }

        Ok(pp.finish())
    }

    #[test]
    fn kanata_logs() {
        for entry in glob("testinput/*.log").unwrap() {
            let path = entry.unwrap();
            let input = std::fs::read(&path).unwrap();
            let out = parse_and_pretty_print(&input).unwrap();
            assert_snapshot!(out);
        }
    }
}
