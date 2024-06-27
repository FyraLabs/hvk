use std::ffi::CString;

pub fn from_raw_array_full<T>(ptr_full: *mut T) -> Box<[T]>
where
    T: Clone,
{
    // Safety: ptr_full is a valid pointer to a null-terminated array of T
    // and the array is terminated by a null value.
    // The array is supposed to be freed by the caller.
    let mut ptr = ptr_full;
    let mut len = 0;

    // Calculate the length of the array
    while !ptr.is_null() {
        len += 1;
        // Safety: ptr is a valid pointer to a null-terminated array of T
        // and the array is terminated by a null value.
        ptr = unsafe { ptr.add(1) };
    }

    // Safety: ptr_full is a valid pointer to a null-terminated array of T
    // and the array is terminated by a null value.
    // The array is supposed to be freed by the caller.
    unsafe { std::slice::from_raw_parts(ptr_full, len) }.into()
}

pub unsafe fn from_raw_cstring_array_full(ptr_full: *mut *mut i8) -> Box<[CString]> {
    // Safety: ptr_full is a valid pointer to a null-terminated array of null-terminated strings
    // and the array is terminated by a null pointer.
    // The array is supposed to be freed by the caller.
    let mut ptr = ptr_full;
    let mut len = 0;

    // Calculate the length of the array
    while !ptr.is_null() {
        len += 1;
        // Safety: ptr is a valid pointer to a null-terminated array of null-terminated strings
        // and the array is terminated by a null pointer.
        ptr = unsafe { ptr.add(1) };
    }

    (0..len)
        .map(|i| {
            // Safety: ptr_full is a valid pointer to a null-terminated array of null-terminated strings
            // and the array is terminated by a null pointer.
            let ptr = unsafe { ptr_full.add(i) };
            // Safety: ptr is a valid pointer to a null-terminated string
            let cstr = unsafe { CString::from_raw(*ptr) };
            cstr
        })
        .collect()
}
