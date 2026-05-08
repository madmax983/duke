#![allow(missing_docs)]
use duke_bytecode::decode;
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
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[test]
fn test_oom_lookupswitch() {
    let mut data = vec![0xab]; // lookupswitch
    data.push(0x00);
    data.push(0x00);
    data.push(0x00); // padding

    // default
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // npairs: 0x3fffffff
    data.push(0x3f);
    data.push(0xff);
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = decode(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}

#[test]
fn test_oom_tableswitch() {
    let mut data = vec![0xaa]; // tableswitch
    data.push(0x00);
    data.push(0x00);
    data.push(0x00); // padding

    // default
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // low: 0
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // high: 0x3fffffff
    data.push(0x3f);
    data.push(0xff);
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = decode(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}

#[test]
fn havoc_cfg_oom() {
    use duke_bytecode::{Instruction, generate_mermaid_cfg};
    let mut instructions = Vec::new();
    let mut offsets = Vec::new();
    // Simulate a massive tableswitch
    for i in 0..1_000_000 {
        offsets.push(i);
    }
    instructions.push((
        0,
        Instruction::Tableswitch {
            default: 0,
            low: 0,
            high: 999_999,
            offsets,
        },
    ));

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = generate_mermaid_cfg(&instructions);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 10_000_000,
        "Allocated {allocated} bytes! OOM triggered in generate_mermaid_cfg"
    );
}

#[test]
#[cfg(feature = "nova")]
fn havoc_basic_block_cfg_oom() {
    use duke_bytecode::{BasicBlock, Instruction, generate_basic_block_cfg};
    let mut instructions = Vec::new();
    let mut offsets = Vec::new();
    // Simulate a massive tableswitch
    for i in 0..1_000_000 {
        offsets.push(i);
    }
    instructions.push((
        0,
        Instruction::Tableswitch {
            default: 0,
            low: 0,
            high: 999_999,
            offsets,
        },
    ));

    let block = BasicBlock {
        start_pc: 0,
        end_pc: 0,
        instructions,
    };

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = generate_basic_block_cfg(&[block]);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 10_000_000,
        "Allocated {allocated} bytes! OOM triggered in generate_basic_block_cfg"
    );
}
