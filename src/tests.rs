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
