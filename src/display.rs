use std::cell::Ref;
use std::fmt::Write;
use std::rc::Rc;
use std::{cell::RefCell, fmt};

use ansi_term::{Color, Style};

use crate::Reporter;
use crate::{
    reporter::{Message, Tattlee},
    Loc,
};

pub struct SourceInfo {
    name: Option<String>,
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
    pub fn new(name: Option<String>, text: Rc<String>) -> Self {
        let mut newlines = Vec::new();
        for (i, c) in text.char_indices() {
            if c == '\n' {
                newlines.push(i)
            }
        }
        Self {
            name,
            text,
            newlines,
        }
    }

    pub fn name(&self) -> &str {
        match &self.name {
            Some(s) => s,
            None => "<none>",
        }
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

    pub fn show_source<W: fmt::Write>(
        &self,
        loc: Loc,
        w: &mut W,
        config: DisplayOptions,
    ) -> fmt::Result {
        let (start_line, end_line) = (self.line_idx(loc.start), self.line_idx(loc.end));
        let start_char = &self.text[self.line_start(start_line)..loc.start]
            .chars()
            .count();
        writeln!(
            w,
            "--> {}:{}:{}",
            self.name(),
            start_line + 1,
            start_char + 1
        )?;
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

pub struct SourceLibrary {
    files: Rc<RefCell<Vec<SourceInfo>>>,
}

impl SourceLibrary {
    pub fn new() -> Self {
        Self {
            files: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn register(&self, name: Option<String>, content: Rc<String>) -> usize {
        let i = self.files.borrow().len();
        self.files.borrow_mut().push(SourceInfo::new(name, content));
        i
    }

    fn file(&self, i: usize) -> Option<Ref<SourceInfo>> {
        Ref::filter_map(self.files.borrow(), |files| files.get(i)).ok()
    }

    fn format_report_to(&self, message: &Message, options: DisplayOptions, out: &mut String) {
        match message {
            Message::Error(e) => {
                writeln!(out, "error[{}]: {}", e.code.short, e.message).unwrap();
                if let Some(l) = e.loc {
                    if let Some(info) = &self.file(l.file) {
                        info.show_source(l, out, options).unwrap()
                    }
                }
            }
            Message::Info(m) => {
                writeln!(out, "info: {m}").unwrap();
            }
        }
    }

    fn format_report(&self, message: &Message, options: DisplayOptions) -> String {
        let mut out = String::new();
        self.format_report_to(message, options, &mut out);
        out
    }

    pub fn format_log(&self, reporter: &Reporter) -> String {
        let mut out = String::new();
        for m in reporter.log().iter() {
            self.format_report_to(m, DisplayOptions::String, &mut out);
        }
        out
    }
}

pub struct StdoutTattlee {
    sources: SourceLibrary,
}

impl Tattlee for StdoutTattlee {
    fn accept_report(&self, message: &Message) {
        println!(
            "{}",
            self.sources
                .format_report(message, DisplayOptions::Terminal)
        )
    }
}

pub struct StderrTattlee {
    sources: SourceLibrary,
}

impl Tattlee for StderrTattlee {
    fn accept_report(&self, message: &Message) {
        eprintln!(
            "{}",
            self.sources
                .format_report(message, DisplayOptions::Terminal)
        )
    }
}

pub struct StringTattlee {
    sources: SourceLibrary,
    out: Rc<RefCell<String>>,
}

impl Tattlee for StringTattlee {
    fn accept_report(&self, message: &Message) {
        write!(
            self.out.borrow_mut(),
            "{}",
            self.sources.format_report(message, DisplayOptions::String)
        )
        .unwrap();
    }
}
