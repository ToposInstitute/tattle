use crate::codes::ErrorCode;
use crate::loc::Loc;

use std::cell::Cell;
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

pub enum Message {
    Error(Error),
    Info(String),
}

#[derive(Clone)]
pub struct Reporter {
    log: Rc<RefCell<Vec<Message>>>,
    errored: Rc<Cell<bool>>,
}

impl Reporter {
    pub fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(Vec::new())),
            errored: Rc::new(Cell::new(false)),
        }
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

    pub fn error_option_loc(&self, loc: Option<Loc>, code: ErrorCode, message: String) {
        let e = Error::new(code, loc, message);
        let m = Message::Error(e);
        self.log.borrow_mut().push(m)
    }

    pub fn info(&self, message: String) {
        let m = Message::Info(message);
        self.log.borrow_mut().push(m);
    }

    pub fn poll(&self) -> Vec<Message> {
        self.log.replace(Vec::new())
    }
}
