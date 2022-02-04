use std::fmt;
use std::io;
use std::string;

/// Custom error type to handle multiple types of errors
/// Needs: Variant for parse errors
#[derive(Debug)]
pub struct TdmsError {
    pub kind: TdmsErrorKind,
}

#[derive(Debug)]
pub enum TdmsErrorKind {
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),
    NoPreviousObject,     // raw_data_index == 0, but no previous object available.
    StringSizeNotDefined, // size is called on TdmsString without first putting a guard in place.
    RawDataTypeNotFound,  // Can't convert from u32 to DataTypeRaw enum variant
    ChannelNotFound,   // Couldn't load the requested data because it does not appear in the file
    ObjectHasNoRawData, // The object doesn't contain any raw data, may want to try just returning the properties.
}

impl fmt::Display for TdmsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            TdmsErrorKind::Io(e) => write!(f, "IO error: {}", e)?,
            TdmsErrorKind::FromUtf8(e) => write!(f, "unable to convert buffer to string: {}", e)?,
            TdmsErrorKind::NoPreviousObject => write!(f, "Raw data index was equal to zero indicating this object has appeared before, but no previous object was recorded. Data may be malformed")?, 
            TdmsErrorKind::StringSizeNotDefined => write!(f, "Calling size directly on a DataTypeRaw::TdmsString is not meaningful. A file read operation is required to either verify total size of string data in a segment, or perform a string read. To perform a string read use 'match_read_string'")?,
            TdmsErrorKind::RawDataTypeNotFound => write!(f, "The parsed u32 did not match a known raw data type")?,
            TdmsErrorKind::ChannelNotFound => write!(f, "The requested channel is not in the channel list, ensure special characters are correctly escaped")?,
            TdmsErrorKind::ObjectHasNoRawData => write!(f, "The requested object does not contain any raw data")?,
        }
        Ok(())
    }
}

// Struggling to introduce the kind method for introspection as the sub types won't let me move out of the kind fields. I guess kind could consume the error?
// impl TdmsError {
//     pub fn kind(&self) -> TdmsErrorKind {
//         // Have to match like this as io::Error and string::FromUtf8Error do not implement clone.
//         match self.kind {
//             TdmsErrorKind::Io => TdmsErrorKind::Io,
//             TdmsErrorKind::FromUtf8 => TdmsErrorKind::FromUtf8,
//             TdmsErrorKind::NoPreviousObject => TdmsErrorKind::NoPreviousObject,
//             TdmsErrorKind::StringSizeNotDefined => TdmsErrorKind::StringSizeNotDefined,
//         }
//     }
// }

impl From<std::io::Error> for TdmsError {
    fn from(err: std::io::Error) -> TdmsError {
        TdmsError {
            kind: TdmsErrorKind::Io(err),
        }
    }
}

impl From<std::string::FromUtf8Error> for TdmsError {
    fn from(err: std::string::FromUtf8Error) -> TdmsError {
        TdmsError {
            kind: TdmsErrorKind::FromUtf8(err),
        }
    }
}

impl std::error::Error for TdmsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
