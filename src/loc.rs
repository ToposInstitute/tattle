#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loc {
    pub start: usize,
    pub end: usize,
    pub file: usize,
}

impl Loc {
    pub fn new(start: usize, end: usize, file: usize) -> Self {
        assert!(start <= end);
        Self { start, end, file }
    }

    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}
