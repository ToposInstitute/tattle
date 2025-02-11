pub struct ErrorDesc {
    pub short: &'static str,
    pub long: &'static str,
}

pub type ErrorCode = &'static ErrorDesc;

impl ErrorDesc {
    pub const fn new(short: &'static str, long: &'static str) -> ErrorDesc {
        ErrorDesc { short, long }
    }
}

#[macro_export]
macro_rules! declare_error {
    ($name:ident, $short:literal, $long:literal) => {
        const $name: &'static $crate::codes::ErrorDesc =
            &$crate::codes::ErrorDesc::new($short, $long);
    };
}
