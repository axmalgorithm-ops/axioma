use core::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    BufferOverflow,
    CorruptStream,
    UnsupportedOperation,
    Overflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BufferOverflow => write!(f, "output buffer overflow"),
            Error::CorruptStream => write!(f, "corrupt compressed stream"),
            Error::UnsupportedOperation => write!(f, "unsupported preprocessor operation"),
            Error::Overflow => write!(f, "arithmetic overflow in range coder"),
        }
    }
}
