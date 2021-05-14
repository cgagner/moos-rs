use core::array::TryFromSliceError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum SerializationError {
    InsufficientSpace(),
    Invalid(),
}

#[derive(Debug)]
pub enum Error {
    Serialization(SerializationError),
    Utf8(Utf8Error),
    General(&'static str),
    Conversion(TryFromSliceError),
}

pub type Result<T> = core::result::Result<T, Error>;

pub const INSUFFICIENT_SPACE_ERROR: Error =
    Error::Serialization(SerializationError::InsufficientSpace());
