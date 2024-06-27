mod error;
mod filesystem;
mod guestfs;

type Result<T> = std::result::Result<T, crate::error::Error>;
use std::{ffi::{CStr, CString}, path::{Path, PathBuf}};
// We should be re-implementing features from std::fs into here
