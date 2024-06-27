use crate::Result;
use libguestfs_sys::guestfs_h;
use std::ffi::{CStr, CString};
mod types;
mod ffi_utils;

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

// todo: take shit from filesystem.rs because i ported it
impl<'a> GuestFs<'a> {
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
                Ok(partitions.into_iter().map(|x| x.to_string_lossy().into_owned()).collect())
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
                Ok(filesystems.into_iter().map(|x| x.to_string_lossy().into_owned()).collect())
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
                let data = unsafe { ffi_utils::from_raw_array_full(data) };
                Ok(data.into_iter().map(|x| x.to_owned() as u8).collect())
            }
        }
    }

}
