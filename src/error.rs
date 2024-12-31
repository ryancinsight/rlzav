use std::fmt;

#[derive(Debug)]
pub enum LzavError {
    Params,
    SourceOutOfBounds,
    DestOutOfBounds,
    ReferenceOutOfBounds,
    DestLengthMismatch,
    UnknownFormat,
}

impl fmt::Display for LzavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LzavError::Params => write!(f, "Invalid parameters"),
            LzavError::SourceOutOfBounds => write!(f, "Source buffer out of bounds"),
            LzavError::DestOutOfBounds => write!(f, "Destination buffer out of bounds"),
            LzavError::ReferenceOutOfBounds => write!(f, "Back-reference out of bounds"),
            LzavError::DestLengthMismatch => write!(f, "Decompressed length mismatch"),
            LzavError::UnknownFormat => write!(f, "Unknown stream format"),
        }
    }
}

impl std::error::Error for LzavError {}

impl From<i32> for LzavError {
    fn from(code: i32) -> Self {
        match code {
            crate::constants::LZAV_E_PARAMS => LzavError::Params,
            crate::constants::LZAV_E_SRCOOB => LzavError::SourceOutOfBounds,
            crate::constants::LZAV_E_DSTOOB => LzavError::DestOutOfBounds,
            crate::constants::LZAV_E_REFOOB => LzavError::ReferenceOutOfBounds,
            crate::constants::LZAV_E_DSTLEN => LzavError::DestLengthMismatch,
            crate::constants::LZAV_E_UNKFMT => LzavError::UnknownFormat,
            _ => LzavError::Params,
        }
    }
}
