use libguestfs_sys::guestfs_stat;

/// A DirEntList is a list of directory entries
pub struct DirEntList {
    pub inner: *mut libguestfs_sys::guestfs_dirent_list,
}

impl Drop for DirEntList {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_dirent_list(self.inner);
        }
    }
}

// todo: Re-implementation of std::fs::File for a singular file
// we should get data from guestfs::guestfs_stat
pub struct Stat {
    inner: *mut guestfs_stat,
}

impl Drop for Stat {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_stat(self.inner);
        }
    }
}

pub struct LvmLv {
    inner: *mut libguestfs_sys::guestfs_lvm_lv,
}

impl Drop for LvmLv {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_lvm_lv(self.inner);
        }
    }
}

pub struct LvmVg {
    inner: *mut libguestfs_sys::guestfs_lvm_vg,
}

impl Drop for LvmVg {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_lvm_vg(self.inner);
        }
    }
}

pub struct LvmPv {
    inner: *mut libguestfs_sys::guestfs_lvm_pv,
}

impl Drop for LvmPv {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_lvm_pv(self.inner);
        }
    }
}

pub struct GuestDirEntry {
    inner: *mut libguestfs_sys::guestfs_dirent,
}

impl Drop for GuestDirEntry {
    fn drop(&mut self) {
        unsafe {
            libguestfs_sys::guestfs_free_dirent(self.inner);
        }
    }
}
