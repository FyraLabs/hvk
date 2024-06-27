// error types powered by thiserror

use std::os::raw::c_int;
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[must_use]
pub enum Error {
    // we gotta map from c int
    #[error("libguestfs error: {0}")]
    GuestFsError(String),
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Invalid CString: {0}")]
    NulError(#[from] std::ffi::NulError),
}
