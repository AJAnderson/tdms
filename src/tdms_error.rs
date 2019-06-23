use std::fmt;
use std::io;
use std::string;

/// Custom error type to handle multiple types of errors
/// Needs: Variant for parse errors
#[derive(Debug)]
pub struct TdmsError {
    pub repr: TdmsErrorKind,
}

#[derive(Debug)]
pub enum TdmsErrorKind {
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),
}

impl fmt::Display for TdmsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.repr {
            TdmsErrorKind::Io(e) => write!(f, "IO error: {}", e)?,
            TdmsErrorKind::FromUtf8(e) => write!(f, "unable to convert buffer to string: {}", e)?,
        }
        Ok(())
    }
}

// impl TdmsError {
//     pub fn kind(&self) -> TdmsErrorKind {
//         match self.repr {
//             TdmsErrorKind::Io(ref e) => TdmsErrorKind::Io(e),
//             TdmsErrorKind::FromUtf8(e) => TdmsErrorKind::FromUtf8(e),
//         }
//     }
// }

impl From<std::io::Error> for TdmsError {
    fn from(err: std::io::Error) -> TdmsError {
        TdmsError {
            repr: TdmsErrorKind::Io(err),
        }
    }
}

impl From<std::string::FromUtf8Error> for TdmsError {
    fn from(err: std::string::FromUtf8Error) -> TdmsError {
        TdmsError {
            repr: TdmsErrorKind::FromUtf8(err),
        }
    }
}

impl std::error::Error for TdmsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
