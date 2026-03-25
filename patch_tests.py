import re

with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    content = f.read()

tests = """
    #[test]
    fn test_archive_ref_from_slot_success() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
        let file_ref = heap.allocate("java/io/File".to_string(), 1);
        heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));
        assert_eq!(archive_ref_from_slot(&heap, obj_ref, 0).unwrap(), Some(file_ref));
    }

    #[test]
    fn test_archive_ref_from_slot_null() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
        heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Reference(None);
        assert_eq!(archive_ref_from_slot(&heap, obj_ref, 0).unwrap(), None);
    }

    #[test]
    fn test_archive_ref_from_slot_none() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
        // Field 1 doesn't exist
        assert_eq!(archive_ref_from_slot(&heap, obj_ref, 1).unwrap(), None);
    }

    #[test]
    fn test_archive_ref_from_slot_type_mismatch() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
        heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(42);
        assert!(matches!(
            archive_ref_from_slot(&heap, obj_ref, 0),
            Err(VmError::TypeMismatch { expected: "Reference", .. })
        ));
    }

    #[test]
    fn test_archive_path_from_slot_none() {
        let mut heap = duke_gc::Heap::new();
        let archive_ref = heap.allocate("org/springframework/boot/loader/launch/JarFileArchive".to_string(), 1);
        heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(None);
        assert_eq!(archive_path_from_slot(&heap, archive_ref, 0).unwrap(), None);
    }
"""

target = "    #[test]\n    fn string_ops_not_equals() {"
content = content.replace(target, tests + "\n" + target)

with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(content)
