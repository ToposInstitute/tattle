use crate::codes::ErrorCode;
use crate::loc::Loc;

use std::cell::{Cell, Ref};
use std::collections::HashMap;
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

pub trait Tattlee {
    fn accept_report(&self, message: &Message);
}

struct Tattlees {
    registered: HashMap<usize, Box<dyn Tattlee>>,
    next: usize,
}

impl Tattlees {
    fn new() -> Self {
        Self {
            registered: HashMap::new(),
            next: 0,
        }
    }

    fn register<T: Tattlee + 'static>(&mut self, t: T) -> usize {
        let i = self.next;
        self.next += 1;
        self.registered.insert(i, Box::new(t));
        i
    }

    fn deregister(&mut self, i: usize) {
        self.registered.remove(&i);
    }

    fn accept_report(&self, message: &Message) {
        for t in self.registered.values() {
            t.accept_report(message);
        }
    }
}

#[derive(Clone)]
pub struct Reporter {
    tattlees: Rc<RefCell<Tattlees>>,
    log: Rc<RefCell<Vec<Message>>>,
    errored: Rc<Cell<bool>>,
}

impl Reporter {
    pub fn new() -> Self {
        Self {
            tattlees: Rc::new(RefCell::new(Tattlees::new())),
            log: Rc::new(RefCell::new(Vec::new())),
            errored: Rc::new(Cell::new(false)),
        }
    }

    pub fn register<T: Tattlee + 'static>(&self, t: T) -> usize {
        self.tattlees.borrow_mut().register(t)
    }

    pub fn deregister<T: Tattlee + 'static>(&self, id: usize) {
        self.tattlees.borrow_mut().deregister(id);
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
        self.tattlees.borrow().accept_report(&m);
        self.log.borrow_mut().push(m)
    }

    pub fn info(&self, message: String) {
        let m = Message::Info(message);
        self.tattlees.borrow().accept_report(&m);
        self.log.borrow_mut().push(m);
    }

    pub fn report_to<T: Tattlee>(&self, t: &T) {
        for m in self.log.borrow().iter() {
            t.accept_report(m);
        }
    }

    pub(crate) fn log(&self) -> Ref<[Message]> {
        Ref::map(self.log.borrow(), |log| log.as_slice())
    }
}
