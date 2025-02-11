use crate::codes::ErrorCode;
use crate::loc::Loc;

use ansi_term::{Color, Style};
use std::cell::Cell;
use std::fmt;
use std::fmt::Write;
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

#[derive(Clone)]
pub enum ReporterOutput {
    Stderr,
    Stdout,
    String(Rc<RefCell<String>>),
    Mem(Rc<RefCell<Vec<Error>>>),
}

pub struct SourceInfo {
    text: String,
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
    pub fn new(text: String) -> Self {
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
    pub fn new(out: ReporterOutput, mut source: String) -> Self {
        // this is used for the EOF
        write!(source, " ").unwrap();
        Self {
            out,
            errored: Rc::new(Cell::new(false)),
            source: Rc::new(SourceInfo::new(source)),
        }
    }

    pub fn errored(&self) -> bool {
        self.errored.get()
    }

    pub fn error<F: Fn(&mut fmt::Formatter) -> fmt::Result>(
        &self,
        loc: Loc,
        code: ErrorCode,
        writer: F,
    ) {
        self.errored.set(true);
        self.error_option_loc(Some(loc), code, writer);
    }

    #[allow(dead_code)]
    pub fn error_unknown_loc<F: Fn(&mut fmt::Formatter) -> fmt::Result>(
        &self,
        code: ErrorCode,
        writer: F,
    ) {
        self.error_option_loc(None, code, writer);
    }

    pub fn error_option_loc<F: Fn(&mut fmt::Formatter) -> fmt::Result>(
        &self,
        loc: Option<Loc>,
        code: ErrorCode,
        writer: F,
    ) {
        match &self.out {
            ReporterOutput::Stdout => {
                println!("error[{}]: {}", code.short, DynWriter(&writer));
                if let Some(loc) = loc {
                    let mut l = String::new();
                    self.source
                        .show_source(loc, &mut l, DisplayOptions::Terminal)
                        .unwrap_or(());
                    println!("{}", &l);
                }
            }
            ReporterOutput::Stderr => {
                eprintln!("error[{}]: {}", code.short, DynWriter(&writer));
                if let Some(loc) = loc {
                    let mut l = String::new();
                    self.source
                        .show_source(loc, &mut l, DisplayOptions::Terminal)
                        .unwrap_or(());
                    eprintln!("{}", &l);
                }
            }
            ReporterOutput::String(s) => {
                let mut s = s.borrow_mut();
                writeln!(s, "error[{}]: {}", code.short, DynWriter(&writer)).unwrap_or(());
                if let Some(loc) = loc {
                    self.source
                        .show_source(loc, &mut *s, DisplayOptions::String)
                        .unwrap_or(());
                }
            }
            ReporterOutput::Mem(v) => {
                let mut msg = String::new();
                write!(&mut msg, "{}", DynWriter(&writer)).unwrap_or(());
                v.borrow_mut().push(Error::new(code, loc, msg));
            }
        };
    }
}

struct DynWriter<'a, F: Fn(&mut fmt::Formatter) -> fmt::Result>(&'a F);

impl<'a, F: Fn(&mut fmt::Formatter) -> fmt::Result> fmt::Display for DynWriter<'a, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let w = self.0;
        w(f)
    }
}
