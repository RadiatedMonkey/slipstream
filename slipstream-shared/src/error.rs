use std::{
    fmt::{self, Display},
    ops::Range,
};
use thiserror::Error;

/// Some type of operation was not supported.
#[derive(Debug, Error, Default)]
pub struct UnsupportedError {
    /// The type of operation that is unsupported.
    pub reason: String,
    pub location: Option<u64>,
}

impl Display for UnsupportedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(location) = self.location {
            write!(
                f,
                "{} is not supported, at location {location}",
                self.reason
            )
        } else {
            write!(f, "{} is not supported", self.reason)
        }
    }
}

#[derive(Debug, Error, Default)]
pub struct CorruptionError {
    pub reason: String,
    pub location: Option<u64>,
}

impl Display for CorruptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = self.location {
            write!(f, "{} at location {}", self.reason, location)
        } else {
            f.write_str(&self.reason)
        }
    }
}

#[derive(Debug, Error, Default)]
pub struct IncorrectFormat {
    pub expected_magic: Vec<u8>,
    pub found_magic: Vec<u8>,
    pub location: Option<u64>,
}

impl Display for IncorrectFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let expected_string = String::from_utf8_lossy(&self.expected_magic);
        let found_string = String::from_utf8_lossy(&self.found_magic);

        if let Some(location) = self.location {
            write!(
                f,
                "expected magic {:?} (`{}`) but found {:?} (`{}`), at location {location}",
                self.expected_magic, expected_string, self.found_magic, found_string
            )
        } else {
            write!(
                f,
                "expected magic {:?} (`{}`) but found {:?} (`{}`)",
                self.expected_magic, expected_string, self.found_magic, found_string
            )
        }
    }
}

#[derive(Debug, Error, Clone, Default)]
pub struct RangeError {
    pub requested: u64,
    pub range: Range<u64>,
    pub location: Option<u64>,
}

impl Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(location) = self.location {
            write!(
                f,
                "{} out of range {}...{}, at location {location}",
                self.requested, self.range.start, self.range.end
            )
        } else {
            write!(
                f,
                "{} out of range {}...{}",
                self.requested, self.range.start, self.range.end
            )
        }
    }
}

#[derive(Debug, Error, Clone, Default)]
pub struct InvalidInputError {
    pub reason: String,
    pub location: Option<u64>,
}

impl Display for InvalidInputError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(location) = self.location {
            write!(f, "{}, at location {location}", self.reason)
        } else {
            f.write_str(&self.reason)
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub struct AssertFailed {
    pub reason: String,
    pub location: Option<u64>,
}

impl Display for AssertFailed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(location) = self.location {
            write!(f, "{}, at location {location}", self.reason)
        } else {
            f.write_str(&self.reason)
        }
    }
}

#[derive(Error, Debug)]
pub enum SlipstreamError {
    #[error("assertion failed: {source}")]
    AssertFailed {
        #[from]
        source: AssertFailed,
    },
    #[error("unsupported: {source}")]
    Unsupported {
        #[from]
        source: UnsupportedError,
    },
    #[error("incorrect format: {source}")]
    IncorrectFormat {
        #[from]
        source: IncorrectFormat,
    },
    #[error("corruption error: {source}")]
    Corrupted {
        #[from]
        source: CorruptionError,
    },
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("invalid utf-8 string: {source}")]
    InvalidString {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error("out of range: {source}")]
    OutOfRange {
        #[from]
        source: RangeError,
    },
    #[error("eframe error: {source}")]
    EframeError {
        #[from]
        source: eframe::Error,
    },
    #[error("invalid input: {source}")]
    InvalidInput {
        #[from]
        source: InvalidInputError,
    },
    #[error("channel has been disconnected")]
    ChannelDisconnect,
    #[error("channel is full")]
    ChannelFull,
}

pub type SlipstreamResult<T> = Result<T, SlipstreamError>;

impl<T> From<futures::channel::mpsc::TrySendError<T>> for SlipstreamError {
    fn from(value: futures::channel::mpsc::TrySendError<T>) -> Self {
        if value.is_disconnected() {
            SlipstreamError::ChannelDisconnect
        } else {
            SlipstreamError::ChannelFull
        }
    }
}

impl From<futures::channel::mpsc::SendError> for SlipstreamError {
    fn from(value: futures::channel::mpsc::SendError) -> Self {
        if value.is_disconnected() {
            SlipstreamError::ChannelDisconnect
        } else {
            SlipstreamError::ChannelFull
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for SlipstreamError {
    fn from(_value: std::sync::mpsc::SendError<T>) -> Self {
        SlipstreamError::ChannelDisconnect
    }
}
