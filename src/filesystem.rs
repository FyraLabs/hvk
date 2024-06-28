use crate::{guestfs::GuestFs, Result};
use std::borrow::{Borrow, BorrowMut};
use std::path::Path;
use std::str::FromStr;
use std::{
    ffi::{CStr, CString},
    str::Bytes,
    sync::{Arc, Mutex},
};

// note: nasty code here
//
// so we have a struct that wraps around the C FFI and then 2 high-level objects that make use of that handle
// `[GuestFileSystem]` and `[GuestFile]`, and other wrapper objects
//
// One small issue: All calls from the C FFI are monolithic, and is expected to run every operation from one big
// god struct. This is not how the Rust filesystem API works, so we want to abstract it away.
//
// We're going to do this by copying the mutable handle to the C FFI struct, then re-creating a new handle to the FFI struct
//
// This is a nasty hack... Might cause some issues later on as it's dropped and re-created!?
//
// Derivative objects of GuestFileSystem should be able to use the handle, but do not drop it unless the main object is dropped
//
// todo: redesign this abstraction for better safety and manageability
//
// suggestion: a `commit()` method that actually writes the changes to the filesystem? this will force all changes to be made at once though
//
// suggestion: some kind of method to also open a write handle to the file somehow and write it live too
//



/// An Augeas device handle for the filesystem
///
pub struct Augeas<'a> {
    handle: Arc<Mutex<GuestFs<'a>>>,
    pub devpath: String,
}

impl Drop for Augeas<'_> {
    fn drop(&mut self) {
        self.handle.lock().unwrap().aug_close().unwrap();
    }
}

/// A representation of a file inside of a `[GuestFileSystem]`
pub struct GuestFile<'a> {
    // a mutable handle to GuestFileSystem
    fs: Arc<*mut GuestFileSystem<'a>>,
    path: String,
}

// horrible...
impl<'a> GuestFile<'a> {

    // somehow create a file handle that's also owned by a GuestFileSystem..., but lets you use a GuestFs handle
    pub fn create(fs: Arc<*mut GuestFileSystem<'a>>, path: String) -> Result<Self> {
        // creating a whole new pointer to a GuestFs just to get a new mutable handle, may cause it to dropped
        // nasty hack!!

        // we want to pass through the handle so it doesn't get dropped early
        //
        //
        
        // horrible
        unsafe {
            fs.as_mut()
        }.expect("nullptr").touch(&path)?;
        Ok(Self { fs, path })
    }
    
    pub fn download(&self, dest: &Path) -> Result<()> {
        <&GuestFs>::from(self).download(&self.path, &dest.display().to_string())
    }
    
    pub fn cat(&self) -> Result<Box<[u8]>> {
        <&GuestFs>::from(self).cat(&self.path)
    }
    
    
}

impl<'a> From<&GuestFile<'a>> for &GuestFs<'a> {
    fn from(value: &GuestFile<'a>) -> Self {
        unsafe { value.fs.as_mut() }.expect("nullptr").inner()
    }
}

/// ACL type for a file or directory
pub enum AclType {
    /// ordinary (access) ACL for any file, directory or other filesystem object.
    Access,
    /// default ACL. Normally this only makes sense if path is a directory.
    Default,
}

impl FromStr for AclType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "access" => Self::Access,
            "default" => Self::Default,
            _ => return Err(s.into()),
        })
    }
}

impl AclType {
    fn to_str(&self) -> &str {
        match self {
            AclType::Access => "access",
            AclType::Default => "default",
        }
    }
}

impl std::fmt::Display for AclType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// High-level wrapper around libguestfs functions
pub struct GuestFileSystem<'a> {
    //guestfs: *mut guestfs,
    inner: GuestFs<'a>,
}

// impl From<GuestFs<'_>> for GuestFileSystem<'_> {
//     fn from(guestfs: GuestFs) -> Self {
//         Self { inner: guestfs }
//     }
// }

impl GuestFileSystem<'_> {
    pub fn new() -> Self {
        Self {
            inner: GuestFs::new(),
        }
    }

    pub(crate) fn inner(&self) -> &GuestFs<'_> {
        &self.inner
    }

    pub(crate) fn handle(&self) -> *mut libguestfs_sys::guestfs_h {
        self.inner.handle()
    }

    /// Add a drive to the disk image
    ///
    /// # Arguments
    ///
    /// * `path` - the path to the drive to add
    pub fn add_drive(&mut self, path: &str) -> Result<()> {
        self.inner.add_drive(path)
    }

    /// List the filesystems on the disk image
    ///
    /// # Returns
    ///
    /// A list of filesystems on the disk image
    pub fn list_filesystems(&self) -> Result<Box<[String]>> {
        self.inner.list_filesystems()
    }

    /// Mount a device to a mountpoint from the disk image
    ///
    /// # Arguments
    ///
    /// * `devpath` - the device path to mount
    pub fn mount(&mut self, devpath: &str, mountpoint: &str) -> Result<()> {
        self.inner.mount(devpath, mountpoint)
    }

    /// Unmount a device from a mountpoint
    ///
    /// # Arguments
    ///
    /// * `mountpoint` - the mountpoint to unmount
    pub fn umount(&mut self, mountpoint: &str) -> Result<()> {
        self.inner.umount(mountpoint)
    }

    /// Creates an empty file at the specified path
    ///
    /// # Arguments
    ///
    /// * `path` - the path to the file to create
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

    /// Get the ACL for a file or directory
    ///
    /// # Arguments
    ///
    /// * `path` - the path to the file or directory
    ///
    /// * `acl_type` - the type of ACL to get
    ///
    /// # Returns
    ///
    /// an ACL string (e.g. "user::rwx,group::r--,other::r--")
    pub fn acl_get_file(&self, path: &str, acl_type: AclType) -> Result<String> {
        self.inner.acl_get_file(path, acl_type.to_str())
    }

    /// Set the ACL for a file or directory
    ///
    /// # Arguments
    ///
    /// * `path` - the path to the file or directory
    /// * `acl_type` - the type of ACL to set
    /// * `acl` - the ACL string (e.g. "user::rwx,group::r--,other::r--")
    pub fn acl_set_file(&self, path: &str, acl_type: AclType, acl: &str) -> Result<()> {
        self.inner.acl_set_file(path, acl_type.to_str(), acl)
    }
}
