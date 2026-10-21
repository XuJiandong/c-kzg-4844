// ckb-c-stdlib intentionally has no allocator. Give the C verifier a private
// malloc ABI backed by the application's Rust global allocator.
use alloc::alloc::{alloc, dealloc};
use core::{alloc::Layout, ffi::c_void, ptr};

const ALIGN: usize = 16;
const HEADER: usize = ALIGN;

#[no_mangle]
unsafe extern "C" fn ckzg_malloc(size: usize) -> *mut c_void {
    let Some(total) = size.checked_add(HEADER) else {
        return ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
        return ptr::null_mut();
    };
    let base = alloc(layout);
    if base.is_null() {
        return ptr::null_mut();
    }
    base.cast::<usize>().write(total);
    base.add(HEADER).cast()
}

#[no_mangle]
unsafe extern "C" fn ckzg_calloc(count: usize, size: usize) -> *mut c_void {
    let Some(bytes) = count.checked_mul(size) else {
        return ptr::null_mut();
    };
    let out = ckzg_malloc(bytes);
    if !out.is_null() {
        ptr::write_bytes(out.cast::<u8>(), 0, bytes);
    }
    out
}

#[no_mangle]
unsafe extern "C" fn ckzg_free(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }
    let base = pointer.cast::<u8>().sub(HEADER);
    let total = base.cast::<usize>().read();
    dealloc(base, Layout::from_size_align_unchecked(total, ALIGN));
}
