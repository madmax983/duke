use duke_runtime::{Slot, VmError};
use duke_gc::Heap;

#[test]
fn havoc_test_sb_delete_start_greater_than_end_panics() {
    let mut heap = Heap::new();
    let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
    heap.get_mut(this_ref).unwrap().string_value = Some("abcdef".to_string());

    let args = vec![
        Slot::Reference(Some(this_ref)),
        Slot::Int(4),
        Slot::Int(2),
    ];

    let mut registry = duke_interpreter::registry::ClassRegistry::default();
    duke_interpreter::stdlib::bootstrap_stdlib(&mut registry, &mut heap);
    let handler = registry.natives().get("java/lang/StringBuilder", "delete", "(II)Ljava/lang/StringBuilder;").unwrap();

    let mut out = std::io::sink();
    let mut control = duke_interpreter::registry::NativeControl::default();

    let result = handler(&args, &mut heap, &mut out, &mut control);
    assert!(matches!(result, Err(VmError::ArrayIndexOutOfBounds { .. })));
}
