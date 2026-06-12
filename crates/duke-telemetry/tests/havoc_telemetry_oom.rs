use duke_telemetry::NativeBoundaryStore;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
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
fn test_telemetry_oom() {
    let mut store = NativeBoundaryStore::default();
    ALLOCATED.store(0, Ordering::SeqCst);

    for i in 0..1_000_000 {
        let class = format!("Class{i}");
        store.record_call(&class, "method", 100, false);
        if ALLOCATED.load(Ordering::SeqCst) > 20_000_000 {
            break;
        }
    }

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 20_000_000,
        "Allocated {allocated} bytes! OOM triggered by unbounded telemetry accumulation."
    );
}
