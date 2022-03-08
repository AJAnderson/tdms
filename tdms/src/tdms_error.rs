use std::fmt;
use std::io;
use std::string;

/// Errors propagated either from low level read operations, or from malformed
/// data in the file
#[derive(Debug)]
pub enum TdmsError {
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),
    NoPreviousObject,
    StringSizeNotDefined,
    RawDataTypeNotFound,
    ChannelNotFound,
    ObjectHasNoRawData,
}

pub type Result<T> = std::result::Result<T, TdmsError>;

impl std::error::Error for TdmsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            TdmsError::Io(ref e) => Some(e),
            TdmsError::FromUtf8(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for TdmsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            TdmsError::Io(e) => {
                write!(f, "IO error: {}", e)?
            },
            TdmsError::FromUtf8(e) => {
                write!(f, "unable to convert buffer to string: {}", e)?
            },
            TdmsError::NoPreviousObject => {
                write!(f, "Raw data index was equal to zero indicating this object has appeared before, 
                but no previous object was recorded. Data may be malformed")?
            }, 
            TdmsError::StringSizeNotDefined => { 
                write!(f, "Calling size directly on a DataTypeRaw::TdmsString is not meaningful.")?
            },
            TdmsError::RawDataTypeNotFound => {
                write!(f, "The parsed u32 did not match a known data type")?
            },
            TdmsError::ChannelNotFound => {
                write!(f, "The requested channel is not in the channel list, ensure special characters are correctly escaped")?
            },
            TdmsError::ObjectHasNoRawData => {
                write!(f, "The requested object does not contain any raw data")?
            },
        }
        Ok(())
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
