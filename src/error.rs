use trackable::error::TrackableError;
use trackable::error::ErrorKind as TrackableErrorKind;

/// This crate specific error type.
#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);

/// The list of the possible error kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Invalid input.
    InvalidInput,

    /// Other error.
    Other,
}
impl TrackableErrorKind for ErrorKind {}
