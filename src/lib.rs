use memchr::{memchr, memchr2};
use std::convert::TryFrom;

#[derive(Debug)]
pub enum ParseError {
    InvalidLogKind,
    InvalidRetireKind,
    InvalidDepKind,
    ExpectedValue,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum LogKind {
    LeftPane = b'0',
    MouseOver = b'1',
    Other = b'2',
}

impl TryFrom<u8> for LogKind {
    type Error = ParseError;

    #[inline]
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(LogKind::LeftPane),
            b'1' => Ok(LogKind::MouseOver),
            b'2' => Ok(LogKind::Other),
            _ => Err(ParseError::InvalidLogKind),
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
    type Error = ParseError;

    #[inline]
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(RetireKind::Retire),
            b'1' => Ok(RetireKind::Flush),
            _ => Err(ParseError::InvalidRetireKind),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DepKind {
    WakeUp = b'0',
}

impl TryFrom<u8> for DepKind {
    type Error = ParseError;

    #[inline]
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(DepKind::WakeUp),
            _ => Err(ParseError::InvalidDepKind),
        }
    }
}

#[derive(Copy, Clone)]
pub struct StrRef(u64);

impl StrRef {
    #[inline]
    pub fn new(offset: u64, len: u16) -> Self {
        Self((offset << 16) | len as u64)
    }

    #[inline]
    pub fn offset(self) -> u64 {
        self.0 >> 16
    }

    #[allow(clippy::len_without_is_empty)]
    #[inline]
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

    #[inline]
    fn rest(&self) -> &'a [u8] {
        &self.input[self.pos..]
    }

    #[inline]
    fn consume(&mut self, n: usize) {
        self.pos += n;
    }

    #[inline]
    fn current(&mut self) -> u8 {
        self.input[self.pos]
    }

    #[inline]
    fn skip_line(&mut self) {
        let rest = self.rest();
        if rest.is_empty() {
            return;
        }

        // Find first line-ending byte
        if let Some(i) = memchr2(b'\n', b'\r', rest) {
            let end = i + 1;
            // Handle Windows CRLF
            if rest[i] == b'\r' && end < rest.len() && rest[end] == b'\n' {
                self.consume(2);
            } else {
                self.consume(1);
            }
        } else {
            // no line ending -> consume the rest
            self.pos = self.input.len();
        }
    }

    #[inline]
    fn next_tab(&mut self) {
        if let Some(i) = memchr(b'\t', self.rest()) {
            self.consume(i + 1);
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
            self.consume(i);
            Ok(v)
        } else {
            Err(ParseError::ExpectedValue)
        }
    }

    fn parse_i32(&mut self) -> Result<i32, ParseError> {
        let c = self.current();
        let mut neg = false;
        if c == b'-' {
            neg = true;
            self.bump();
        } else if c == b'+' {
            self.bump();
        }
        let num = self.parse_u64()? as i32;
        if neg { Ok(-num) } else { Ok(num) }
    }

    fn parse_u8(&mut self) -> Result<u8, ParseError> {
        self.parse_u64().map(|v| v as u8)
    }

    fn parse_text(&mut self) -> StrRef {
        let start = self.pos as u64;
        let len = memchr(b'\n', self.rest()).unwrap_or(self.rest().len());
        let res = StrRef::new(start, len as u16);
        self.consume(len);
        res
    }

    fn parse_header(&mut self) -> Result<Command, ParseError> {
        let kanata = b"Kanata\t";
        if !self.rest().starts_with(kanata) {
            return Err(ParseError::InvalidLogKind); // Maybe add a specific Header error?
        }
        self.consume(kanata.len());
        let version = self.parse_u8().unwrap(); // version
        self.skip_line();
        Ok(Command::Kanata { version })
    }

    fn parse_c(&mut self) -> Result<Command, ParseError> {
        self.bump(); // C
        let abs = self.current() == b'=';
        if abs {
            self.bump();
        }
        self.next_tab();
        let value = self.parse_i32().unwrap();
        self.skip_line();
        Ok(Command::Cycle { abs, value })
    }

    fn parse_i(&mut self) -> Result<Command, ParseError> {
        self.bump(); // I
        self.next_tab();
        let id_file = self.parse_u8().unwrap();
        self.next_tab();
        let id_sim = self.parse_u8().unwrap();
        self.next_tab();
        let thread = self.parse_u8().unwrap();
        self.skip_line();
        Ok(Command::Instruction {
            id_in_file: id_file,
            id_in_sim: id_sim,
            thread_id: thread,
        })
    }

    fn parse_l(&mut self) -> Result<Command, ParseError> {
        self.bump(); // L
        self.next_tab();
        let id = self.parse_u8().unwrap();
        self.next_tab();
        let kind = LogKind::try_from(self.current())?;
        self.bump();
        self.next_tab();
        let text = self.parse_text();
        self.skip_line();
        Ok(Command::Log { id, kind, text })
    }

    fn parse_s(&mut self) -> Result<Command, ParseError> {
        self.bump(); // S
        self.next_tab();
        let id = self.parse_u8().unwrap();
        self.next_tab();
        let lane = self.parse_u8().unwrap();
        self.next_tab();
        let name = self.parse_text();
        self.skip_line();
        Ok(Command::Pipeline {
            start: true,
            id,
            lane_id: lane,
            name,
        })
    }

    fn parse_e(&mut self) -> Result<Command, ParseError> {
        self.bump(); // E
        self.next_tab();
        let id = self.parse_u8().unwrap();
        self.next_tab();
        let lane = self.parse_u8().unwrap();
        self.next_tab();
        let name = self.parse_text();
        self.skip_line();
        Ok(Command::Pipeline {
            start: false,
            id,
            lane_id: lane,
            name,
        })
    }

    fn parse_r(&mut self) -> Result<Command, ParseError> {
        self.bump(); // R
        self.next_tab();
        let id = self.parse_u8().unwrap();
        self.next_tab();
        let retire = self.parse_u8().unwrap();
        self.next_tab();
        let kind = RetireKind::try_from(self.current())?;
        self.bump();
        self.skip_line();
        Ok(Command::Retire { id, retire, kind })
    }

    fn parse_w(&mut self) -> Result<Command, ParseError> {
        self.bump(); // W
        self.next_tab();
        let c = self.parse_u8().unwrap();
        self.next_tab();
        let p = self.parse_u8().unwrap();
        self.next_tab();
        let kind = DepKind::try_from(self.current())?;
        self.bump();
        self.skip_line();
        Ok(Command::Dep {
            consumer_id: c,
            producer_id: p,
            kind,
        })
    }

    #[inline]
    fn bump(&mut self) {
        self.consume(1);
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Command, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.input.len() {
            let b = self.current();
            let res = match b {
                b'K' => self.parse_header(),
                b'C' => self.parse_c(),
                b'I' => self.parse_i(),
                b'L' => self.parse_l(),
                b'S' => self.parse_s(),
                b'E' => self.parse_e(),
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

        #[inline]
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
