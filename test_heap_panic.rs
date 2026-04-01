use duke_gc::Heap;

fn main() {
    let mut heap = Heap::new();
    // Simulate user passing u64::MAX without OLD_BIT, which will panic on 32-bit platforms.
    // However, on a 64-bit platform, it would try to allocate or index into `young` which has very few elements,
    // leading to index out of bounds panic, since `young.get(idx)` is used but `idx` is huge.
    // Wait, the index out of bounds doesn't panic if they use `get(idx)`, it returns `None` and maps to `VmError::InvalidRef`.
    // Let's check `get` method in `Heap`.
}
