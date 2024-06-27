use crate::{guestfs::GuestFs, Result};
use std::{ffi::{CStr, CString}, str::Bytes};
// use libguestfs_sys
/// Helper function to convert a null-terminated array of strings to a Vec<&str>
/// This is useful for converting the output of libguestfs functions that return
/// a null-terminated array of strings, obviously.
///
/// Very useful for array output from libguestfs functions
#[must_use]
fn null_terminated_array_to_vec<'a>(array: *mut *mut i8) -> Vec<&'a str> {
    let mut vec = Vec::new();
    let mut i = 0;
    loop {
        let ptr = unsafe { *array.offset(i) };
        if ptr.is_null() {
            break;
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        vec.push(cstr.to_str().unwrap());
        i += 1;
    }
    vec
}


/// A representation of a file inside of a `[GuestFileSystem]`
pub struct GuestFile<'a> {
    handle: GuestFs<'a>,
    path: String,
}

impl GuestFile<'_> {
    pub fn create(&self, path: &str) -> Result<Self> {
        self.handle.touch(path)?;
        Ok(Self {
            handle: self.handle,
            path: path.to_string(),
        })
    }
}

/// High-level wrapper around libguestfs functions
pub struct GuestFileSystem<'a> {
    //guestfs: *mut guestfs,
    inner: GuestFs<'a>,
}

impl GuestFileSystem<'_> {
    pub fn new() -> Self {
        Self {
            inner: GuestFs::new(),
        }
    }
    
    
    /// Add a drive to the disk image
    pub fn add_drive(&mut self, path: &str) -> Result<()> {
        self.inner.add_drive(path)
    }

    /// List the filesystems on the disk image
    pub fn list_filesystems(&self) -> Result<Box<[String]>> {
        self.inner.list_filesystems()
    }

    /// Mount a device to a mountpoint from the disk image
    pub fn mount(&mut self, devpath: &str, mountpoint: &str) -> Result<()> {
        self.inner.mount(devpath, mountpoint)
    }

    
    /// Unmount a device from a mountpoint
    pub fn umount(&mut self, mountpoint: &str) -> Result<()> {
        self.inner.umount(mountpoint)
    }
    
    
    /// Creates an empty file at the specified path
    pub fn touch(&mut self, path: &str) -> Result<()> {
        self.inner.touch(path)
    }
    
    /// Shutdown the guestfs appliance
    /// This is called automatically when the GuestFs object is dropped,
    /// but you can call it manually if you'd like to handle errors
    /// during shutdown.
    pub fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown()
    }
}
