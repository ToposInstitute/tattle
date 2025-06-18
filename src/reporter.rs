use crate::codes::ErrorCode;
use crate::loc::Loc;

use ansi_term::{Color, Style};
use std::cell::Cell;
use std::fmt;
use std::fmt::Write;
use std::io;
use std::{cell::RefCell, rc::Rc};

pub struct Error {
    pub code: ErrorCode,
    pub loc: Option<Loc>,
    pub message: String,
}

impl Error {
    fn new(code: ErrorCode, loc: Option<Loc>, message: String) -> Self {
        Self { code, loc, message }
    }
}

#[derive(Clone, Copy)]
pub enum Console {
    Stderr,
    Stdout,
    None,
}

impl Console {
    fn sink(&self) -> Option<Box<dyn io::Write>> {
        match self {
            Console::Stderr => Some(Box::new(io::stderr())),
            Console::Stdout => Some(Box::new(io::stdout())),
            Console::None => None,
        }
    }
}

pub enum Message {
    Error(Error),
    Info(String),
}

#[derive(Clone)]
pub struct ReporterOutput {
    console: Console,
    log: Rc<RefCell<Vec<Message>>>,
}

impl ReporterOutput {
    fn new() -> Self {
        ReporterOutput {
            console: Console::None,
            log: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

pub struct SourceInfo {
    text: Rc<String>,
    newlines: Vec<usize>,
}

pub enum DisplayOptions {
    Terminal,
    String,
}

struct Repeated(usize, char);

impl fmt::Display for Repeated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.0 {
            write!(f, "{}", self.1)?
        }
        Ok(())
    }
}

impl SourceInfo {
    pub fn new(text: Rc<String>) -> Self {
        let mut newlines = Vec::new();
        for (i, c) in text.char_indices() {
            if c == '\n' {
                newlines.push(i)
            }
        }
        Self { text, newlines }
    }

    fn line_idx(&self, bytepos: usize) -> usize {
        self.newlines.partition_point(|i| *i < bytepos)
    }

    // Returns the byte position of the first byte in the nth line
    fn line_start(&self, n: usize) -> usize {
        if n == 0 {
            0
        } else if n > self.newlines.len() {
            self.text.len() - 1
        } else {
            (self.text.len() - 1).min(self.newlines[n - 1] + 1)
        }
    }

    // Returns the byte position of the newline that ends the nth line
    // In the case of the last line, this simply returns the length of source.
    // Intended to be used as line(s) = &source[line_start(s)..line_end(s)]
    fn line_end(&self, n: usize) -> usize {
        if n < self.newlines.len() {
            self.newlines[n]
        } else {
            self.text.len()
        }
    }

    fn show_source<W: fmt::Write>(
        &self,
        loc: Loc,
        w: &mut W,
        config: DisplayOptions,
    ) -> fmt::Result {
        let (start_line, end_line) = (self.line_idx(loc.start), self.line_idx(loc.end));
        let style = Style::new().bold().underline().fg(Color::Red);
        for line in start_line..=end_line {
            let (s, e) = (self.line_start(line), self.line_end(line));
            let (hs, he) = (s.max(loc.start), e.min(loc.end));
            match config {
                DisplayOptions::String => {
                    writeln!(w, "{:4>}| {}", line + 1, &self.text[s..e],)?;
                    writeln!(
                        w,
                        "{:4>}| {}{}",
                        line + 1,
                        Repeated(hs - s, ' '),
                        Repeated(he - hs, '^')
                    )?;
                }
                DisplayOptions::Terminal => {
                    writeln!(
                        w,
                        "{:4>}| {}{}{}",
                        line + 1,
                        &self.text[s..hs],
                        style.paint(&self.text[hs..he]),
                        &self.text[he..e]
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Reporter {
    out: ReporterOutput,
    errored: Rc<Cell<bool>>,
    source: Rc<SourceInfo>,
}

impl Reporter {
    pub fn new(source: Rc<String>) -> Self {
        Self {
            out: ReporterOutput::new(),
            errored: Rc::new(Cell::new(false)),
            source: Rc::new(SourceInfo::new(source)),
        }
    }

    pub fn enable_stdout(mut self) -> Self {
        self.out.console = Console::Stdout;
        self
    }

    pub fn enable_stderr(mut self) -> Self {
        self.out.console = Console::Stderr;
        self
    }

    pub fn errored(&self) -> bool {
        self.errored.get()
    }

    pub fn error(&self, loc: Loc, code: ErrorCode, message: String) {
        self.errored.set(true);
        self.error_option_loc(Some(loc), code, message);
    }

    pub fn error_unknown_loc(&self, code: ErrorCode, message: String) {
        self.error_option_loc(None, code, message);
    }

    fn write_io(&self, e: &Error) {
        self.out.console.sink().map(|mut io| {
            writeln!(io, "error[{}]: {}", e.code.short, e.message).unwrap();
            if let Some(loc) = e.loc {
                let mut l = String::new();
                self.source
                    .show_source(loc, &mut l, DisplayOptions::Terminal)
                    .unwrap_or(());
                writeln!(io, "{}", &l).unwrap();
            }
        });
    }

    fn write_fmt(&self, e: &Error, f: &mut impl fmt::Write) {
        writeln!(f, "error[{}]: {}", e.code.short, e.message).unwrap();
        if let Some(loc) = e.loc {
            self.source
                .show_source(loc, f, DisplayOptions::String)
                .unwrap_or(());
        }
    }

    pub fn error_option_loc(&self, loc: Option<Loc>, code: ErrorCode, message: String) {
        let e = Error::new(code, loc, message);
        self.write_io(&e);
        self.out.log.borrow_mut().push(Message::Error(e))
    }

    pub fn info(&self, message: String) {
        self.out.console.sink().map(|mut io| {
            writeln!(io, "{}", message).unwrap();
        });
        self.out.log.borrow_mut().push(Message::Info(message));
    }

    pub fn report(&self) -> String {
        let mut out = String::new();
        for m in self.out.log.borrow().iter() {
            match m {
                Message::Error(e) => self.write_fmt(e, &mut out),
                Message::Info(s) => {
                    writeln!(&mut out, "{}", s).unwrap();
                }
            }
        }
        // remove the last newline
        out.pop();
        out
    }
}
