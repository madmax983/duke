#![allow(missing_docs)]
use duke_loader::{ClassLoader, DirectoryLoader};
use std::path::PathBuf;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
const OOM_LIMIT: usize = 10 * 1024 * 1024; // 10MB limit

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        if current + layout.size() > OOM_LIMIT {
            // we panic instead of returning null to simulate a controlled crash during testing
            // Wait, returning null *is* the Rust way to OOM.
            // In the previous run, returning null failed the 20MB allocation in `let long_name = "A".repeat(20 * 1024 * 1024);` BEFORE `find_class`!
            // THAT is why it crashed!!!
            return std::ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[test]
fn test_directory_loader_oom() {
    let loader = DirectoryLoader::new(PathBuf::from("/tmp"));

    // We must ensure the `A.repeat` doesn't OOM.
    // So we use a long string just slightly under 10MB to test if the loader allocates.
    // No, wait, if we have a string that triggers `NameTooLong`, it should just return the error immediately without allocating.
    // Let's allocate 5MB for the string, which is fine under the 10MB limit.
    // Then call find_class. If find_class allocates 5MB again, it will OOM!
    let long_name = "A".repeat(6 * 1024 * 1024); // 6MB string
    assert!(matches!(
        loader.find_class(&long_name),
        Err(duke_loader::Error::NameTooLong)
    ));
}
