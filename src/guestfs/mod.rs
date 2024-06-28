use crate::Result;
use libguestfs_sys::guestfs_h;
use std::{
    borrow::Borrow,
    ffi::{CStr, CString},
    io::{Cursor, Read},
    path::Path,
};
use types::DirEntList;
mod ffi_utils;
mod types;

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

// guestfs functions return 0 on success, -1 on error

// https://www.libguestfs.org/guestfs.3.html#api-calls
// Main GuestFS struct and its associated functions

/// GuestFS struct that wraps the libguestfs handle
pub struct GuestFs<'a> {
    handle: *mut guestfs_h,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl Drop for GuestFs<'_> {
    fn drop(&mut self) {
        unsafe {
            // unconditionally shutdown and ignore all errors
            // todo: log errors

            let _ = self.shutdown();

            libguestfs_sys::guestfs_close(self.handle);
        }
    }
}

impl From<*mut guestfs_h> for GuestFs<'_> {
    fn from(handle: *mut guestfs_h) -> Self {
        Self {
            handle,
            _marker: std::marker::PhantomData,
        }
    }
}

// todo: take from filesystem.rs because i ported it
impl<'a> GuestFs<'a> {
    pub(crate) fn handle(&self) -> *mut guestfs_h {
        self.handle
    }
    
    #[doc(alias = "create", "guestfs_create")]
    pub fn new() -> Self {
        let handle = unsafe { libguestfs_sys::guestfs_create() };
        Self {
            handle,
            _marker: std::marker::PhantomData,
        }
    }

    #[must_use]
    #[inline]
    fn parse_error(&self, retcode: i32) -> crate::error::Error {
        let cstr = unsafe { CStr::from_ptr(libguestfs_sys::guestfs_last_error(self.handle)) };
        crate::error::Error::GuestFsError(format!(
            "returned {}: {}",
            retcode,
            cstr.to_string_lossy()
        ))
    }

    #[must_use]
    #[inline]
    fn last_error_number(&self) -> i32 {
        unsafe { libguestfs_sys::guestfs_last_errno(self.handle) }
    }

    #[must_use]
    #[inline]
    fn wrap_error(&self, retcode: i32) -> Result<()> {
        if retcode != 0 {
            Err(self.parse_error(retcode))
        } else {
            Ok(())
        }
    }

    /// Adds a new drive
    pub fn add_drive(&mut self, path: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_add_drive(self.handle, CString::new(path)?.as_ptr())
        })
    }
    /// Shutdown the libguestfs appliance
    pub fn shutdown(&self) -> Result<()> {
        self.wrap_error(unsafe { libguestfs_sys::guestfs_shutdown(self.handle) })
    }

    pub fn mount(&mut self, devpath: &str, mountpoint: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_mount(
                self.handle,
                CString::new(devpath)?.as_ptr(),
                CString::new(mountpoint)?.as_ptr(),
            )
        })
    }

    /// Unmount a device from a mountpoint
    pub fn umount(&mut self, mountpoint: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_umount(self.handle, CString::new(mountpoint)?.as_ptr())
        })
    }
    /// Creates an empty file at the specified path
    pub fn touch(&mut self, path: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_touch(self.handle, CString::new(path)?.as_ptr())
        })
    }

    /// List partitions inside the disk image
    ///
    /// Returns a list of device paths of the partitions, e.g. /dev/sda1
    pub fn list_partitions(&self) -> Result<Box<[String]>> {
        match unsafe { libguestfs_sys::guestfs_list_partitions(self.handle) } {
            partitions if partitions.is_null() => Err(self.parse_error(self.last_error_number())),
            partitions => {
                let partitions = unsafe { ffi_utils::from_raw_cstring_array_full(partitions) };
                Ok(partitions
                    .into_iter()
                    .map(|x| x.to_string_lossy().into_owned())
                    .collect())
            }
        }
    }

    /// List filesystems inside the disk image
    ///
    pub fn list_filesystems(&self) -> Result<Box<[String]>> {
        match unsafe { libguestfs_sys::guestfs_list_filesystems(self.handle) } {
            filesystems if filesystems.is_null() => Err(self.parse_error(self.last_error_number())),
            filesystems => {
                let filesystems = unsafe { ffi_utils::from_raw_cstring_array_full(filesystems) };
                Ok(filesystems
                    .into_iter()
                    .map(|x| x.to_string_lossy().into_owned())
                    .collect())
            }
        }
    }

    /// Concatenate a file and return its contents as an array of bytes
    pub fn cat(&self, path: &str) -> Result<Box<[u8]>> {
        // // let mut size: u64 = 0;
        // let data = unsafe {
        //     libguestfs_sys::guestfs_cat(self.handle, CString::new(path)?.as_ptr())
        // };
        // if data.is_null() {
        //     return Err(self.parse_error(self.last_error_number()));
        // }

        // // Convert to a Vec<u8>
        // // todo: am i doing this right?
        // let vec = unsafe { std::slice::from_raw_parts(data, 0) };

        // Ok(vec.into_iter().map(|x| *x as u8).collect())
        match unsafe { libguestfs_sys::guestfs_cat(self.handle, CString::new(path)?.as_ptr()) } {
            data if data.is_null() => Err(self.parse_error(self.last_error_number())),
            data => {
                let data = ffi_utils::from_raw_array_full(data);
                Ok(data.into_iter().map(|x| x.to_owned() as u8).collect())
            }
        }
    }

    /// Read a file and return a pointer to a Read trait object
    ///
    /// This function is different from [`Self::cat`] in that it can correctly handle files that
    /// contain embedded ASCII NUL characters, and it returns a `Cursor<&[i8]>` instead of a `Vec<u8>`.
    pub fn read_file(&self, path: &str, buf_size: &mut usize) -> Result<Cursor<&[i8]>> {
        let buf = unsafe {
            libguestfs_sys::guestfs_read_file(self.handle, CString::new(path)?.as_ptr(), buf_size)
        };

        if buf.is_null() {
            Err(self.parse_error(self.last_error_number()))
        } else {
            Ok(Cursor::new(unsafe {
                std::slice::from_raw_parts(buf, *buf_size)
            }))
        }
    }

    /// Read a file and return lines as an iterator
    pub fn read_lines(&self, path: &str) -> Result<Box<[String]>> {
        match unsafe {
            libguestfs_sys::guestfs_read_lines(self.handle, CString::new(path)?.as_ptr())
        } {
            lines if lines.is_null() => Err(self.parse_error(self.last_error_number())),
            lines => {
                let lines = unsafe { ffi_utils::from_raw_cstring_array_full(lines) };
                Ok(lines
                    .into_iter()
                    .map(|x| x.to_string_lossy().into_owned())
                    .collect())
            }
        }
    }

    /// Create a directory at the specified path
    pub fn mkdir(&self, path: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_mkdir(self.handle, CString::new(path)?.as_ptr())
        })
    }

    /// Create a symbolic link to a specified target in the filesystem
    pub fn ln_s(&self, target: &str, linkpath: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_ln_s(
                self.handle,
                CString::new(target)?.as_ptr(),
                CString::new(linkpath)?.as_ptr(),
            )
        })
    }

    /// List subdirectories of a directory
    pub fn readdir(&self, path: &str) -> Result<DirEntList> {
        match unsafe { libguestfs_sys::guestfs_readdir(self.handle, CString::new(path)?.as_ptr()) }
        {
            entries if entries.is_null() => Err(self.parse_error(self.last_error_number())),
            entries => Ok(DirEntList { inner: entries }),
        }
    }

    /// Download a file from the disk image to the host
    pub fn download(&self, path: &str, dest: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_download(
                self.handle,
                CString::new(path)?.as_ptr(),
                CString::new(dest)?.as_ptr(),
            )
        })
    }
    /// Get ACL type of a file
    /// The acltype parameter may be:
    /// access
    ///     Return the ordinary (access) ACL for any file, directory or other filesystem object.
    /// default
    ///
    ///     Return the default ACL. Normally this only makes sense if path is a directory.
    ///
    // todo: high level function: enum for acltype
    pub fn acl_get_file(&self, path: &str, acltype: &str) -> Result<String> {
        match unsafe {
            libguestfs_sys::guestfs_acl_get_file(
                self.handle,
                CString::new(path)?.as_ptr(),
                CString::new(acltype)?.as_ptr(),
            )
        } {
            acl if acl.is_null() => Err(self.parse_error(self.last_error_number())),
            acl => {
                let acl = unsafe { CString::from_raw(acl) };
                Ok(acl.to_string_lossy().into_owned())
            }
        }
    }

    pub fn acl_set_file(&self, path: &str, acltype: &str, acl: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_acl_set_file(
                self.handle,
                CString::new(path)?.as_ptr(),
                CString::new(acltype)?.as_ptr(),
                CString::new(acl)?.as_ptr(),
            )
        })
    }

    pub fn add_drive_ro(&self, path: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_add_drive_ro(self.handle, CString::new(path)?.as_ptr())
        })
    }

    // This function accepts a list of domain arguments which are variable args
    // time to use varargs
    // variyak time
    // pub fn add_domain(&self, dom: &str) -> Result<()> {
    //     self.wrap_error(unsafe {
    //         libguestfs_sys::guestfs_add_domain(self.handle, CString::new(dom)?.as_ptr())
    //     })
    // } 
    //
    /// Clear the Augeas node (similar to `augtool clear`)
    pub fn aug_clear(&mut self, augpath: &str) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_aug_clear(self.handle, CString::new(augpath)?.as_ptr())
        })
    }
    
    /// Close the Augeas handle
    /// This function should be called automatically in the idiomatic interface, but we're exposing it here so we can use it
    pub fn aug_close(&mut self) -> Result<()> {
        self.wrap_error(unsafe {
            libguestfs_sys::guestfs_aug_close(self.handle)
        })
    }
    
    
}
