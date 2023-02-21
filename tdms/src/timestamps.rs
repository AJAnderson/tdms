use crate::tdms_error::{Result, TdmsError};
use chrono::prelude::*;
use chrono::{Local, LocalResult};
use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct TimeStamp {
    pub epoch: i64,
    pub radix: u64,
}

const FRACTIONS_PER_NS: u64 = 2 ^ 64 / 10 ^ 9;

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}\t{}", self.epoch, self.radix)?;

        Ok(())
    }
}

impl TimeStamp {
    pub fn to_local_time(&mut self) -> Result<DateTime<Local>> {
        let nanoseconds = (self.radix / FRACTIONS_PER_NS) as u32;
        match Local.timestamp_opt(self.epoch, nanoseconds) {
            LocalResult::Single(timestamp) => Ok(timestamp),
            LocalResult::None => Err(TdmsError::MalformedTimestamp {
                seconds: self.epoch,
                nano: nanoseconds,
            }),
            LocalResult::Ambiguous(_, _) => Err(TdmsError::AmbiguousTimestamp {
                seconds: self.epoch,
                nano: nanoseconds,
            }),
        }
    }
}
