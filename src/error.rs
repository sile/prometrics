//TODO: use std::sync::PoisonError;
use std::sync::mpsc::SendError;
use trackable::error::TrackableError;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};

/// This crate specific error type.
#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);
// impl<T> From<PoisonError<T>> for Error {
//     fn from(f: PoisonError<T>) -> Self {
//         ErrorKind::Other.cause(f.to_string()).into()
//     }
// }
impl<T> From<SendError<T>> for Error {
    fn from(f: SendError<T>) -> Self {
        ErrorKind::Other.cause(f.to_string()).into()
    }
}

/// The list of the possible error kinds
#[derive(Debug, Clone)]
pub enum ErrorKind {
    InvalidInput,
    Other,
}
impl TrackableErrorKind for ErrorKind {}
