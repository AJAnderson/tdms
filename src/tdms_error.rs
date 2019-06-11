use std::io;
use std::string;

/// Custom error type to handle multiple types of errors
/// Needs: Variant for parse errors
#[derive(Debug)]
pub enum TdmsError {
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),
    Custom(String),
}

// QUESTION: How to I convert this to &str? Do I want to?
impl From<String> for TdmsError {
    fn from(msg: String) -> TdmsError {
        TdmsError::Custom(msg)
    }
}

impl From<std::io::Error> for TdmsError {
    fn from(err: std::io::Error) -> TdmsError {
        TdmsError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for TdmsError {
    fn from(err: std::string::FromUtf8Error) -> TdmsError {
        TdmsError::FromUtf8(err)
    }
}