use ansi_term::{Color, Style};
use std::{fmt, io};

use crate::{reporter::Message, Loc, Reporter};

pub struct SourceInfo<'a> {
    name: Option<&'a str>,
    text: &'a str,
    newlines: Vec<usize>,
}

#[derive(Clone, Copy)]
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

impl<'a> SourceInfo<'a> {
    pub fn new(name: Option<&'a str>, text: &'a str) -> Self {
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

    pub fn write_fmt(
        &self,
        w: &mut impl fmt::Write,
        m: &Message,
        options: DisplayOptions,
    ) -> fmt::Result {
        match m {
            Message::Error(e) => {
                writeln!(w, "error[{}]: {}", e.code.short, e.message)?;
                if let Some(loc) = e.loc {
                    self.show_source(loc, w, options)?;
                }
            }
            Message::Info(m) => {
                writeln!(w, "info: {m}")?;
            }
        }
        Ok(())
    }

    pub fn extract_report_to(
        &self,
        w: &mut impl fmt::Write,
        r: Reporter,
        options: DisplayOptions,
    ) -> fmt::Result {
        for m in r.poll().into_iter() {
            self.write_fmt(w, &m, options)?;
        }
        Ok(())
    }

    pub fn extract_report_to_io(
        &self,
        w: &mut impl io::Write,
        r: Reporter,
        options: DisplayOptions,
    ) -> io::Result<()> {
        let mut buf = String::new();
        for m in r.poll().into_iter() {
            self.write_fmt(&mut buf, &m, options)
                .expect("failed to format message {m}");
            w.write(buf.as_bytes())?;
        }
        w.flush()?;
        Ok(())
    }

    pub fn extract_report_to_string(&self, r: Reporter) -> String {
        let mut out = String::new();
        self.extract_report_to(&mut out, r, DisplayOptions::String)
            .unwrap();
        out
    }
}
