//! Cadence core error type.
//!
//! All parsing and validation paths in Cadence return this error type instead
//! of panicking. Errors are deliberately small and allocation-light so that
//! malformed input can be rejected cheaply.

use std::fmt;

/// Maximum number of bytes accepted by any single text parser (16 MiB).
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A human readable message describing a recoverable failure.
    Message(String),
    /// A parser limit (input length, nesting, node count, ...) was exceeded.
    Limit(&'static str),
    /// Input ended before the structure was complete.
    UnexpectedEof,
    /// A numeric or symbolic value was outside the supported range.
    OutOfRange(String),
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Message(message.into())
    }

    pub fn out_of_range(what: impl Into<String>) -> Self {
        Error::OutOfRange(what.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(message) => f.write_str(message),
            Error::Limit(limit) => write!(f, "input limit exceeded: {limit}"),
            Error::UnexpectedEof => f.write_str("unexpected end of input"),
            Error::OutOfRange(what) => write!(f, "value out of range: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        Error::msg("formatting failed")
    }
}

pub type Result<T> = std::result::Result<T, Error>;
