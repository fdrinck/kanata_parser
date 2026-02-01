use crate::parser::ParseErrorKind;

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
        version: u32,
    },
    Cycle {
        abs: bool,
        value: i32,
    },
    Instruction {
        id_in_file: u32,
        id_in_sim: u32,
        thread_id: u32,
    },
    Log {
        id: u32,
        kind: LogKind,
        text: StrRef,
    },
    Pipeline {
        start: bool,
        id: u32,
        lane_id: u32,
        name: StrRef,
    },
    Retire {
        id: u32,
        retire: u32,
        kind: RetireKind,
    },
    Dep {
        consumer_id: u32,
        producer_id: u32,
        kind: DepKind,
    },
}
