#[test]
fn should_return_error_when_reading_closed_or_invalid_file() {
    let mut gc = Heap::new();
    let err = gc.read_host_file_byte(999).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_writing_closed_or_invalid_file() {
    let mut gc = Heap::new();
    let err = gc.write_host_file_byte(999, 65).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_reading_from_writer() {
    let mut gc = Heap::new();
    let path = std::env::temp_dir().join("test_write.txt");
    let id = gc.open_host_output_file(&path).unwrap();
    let err = gc.read_host_file_byte(id).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
    gc.close_host_file(id);
    let _ = std::fs::remove_file(path);
}

#[test]
fn should_return_error_when_writing_to_reader() {
    let mut gc = Heap::new();
    let path = std::env::temp_dir().join("test_read.txt");
    std::fs::write(&path, b"hello").unwrap();
    let id = gc.open_host_input_file(&path).unwrap();
    let err = gc.write_host_file_byte(id, 65).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
    gc.close_host_file(id);
    let _ = std::fs::remove_file(path);
}

#[test]
fn should_return_error_when_opening_non_existent_file() {
    let mut gc = Heap::new();
    let path = std::env::temp_dir().join("definitely_does_not_exist_1234.txt");
    let err = gc.open_host_input_file(&path).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/FileNotFoundException")
    );
}

#[test]
fn should_return_error_when_spawning_empty_command() {
    let mut gc = Heap::new();
    let err = gc.spawn_host_process(&[], None).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_cache_process_exit_code() {
    let mut gc = Heap::new();
    let process = gc
        .spawn_host_process(&["echo".to_string(), "hello".to_string()], None)
        .unwrap();
    let code = gc.wait_host_process(process.process_id).unwrap();
    assert_eq!(code, 0);
    // Should use cache
    let code2 = gc.wait_host_process(process.process_id).unwrap();
    assert_eq!(code2, 0);
    // Try wait should use cache
    let code3 = gc.try_host_process_exit_value(process.process_id).unwrap();
    assert_eq!(code3, Some(0));
    // Destroy on already exited should be ok
    gc.destroy_host_process(process.process_id).unwrap();
}

#[test]
fn should_return_error_when_waiting_invalid_process() {
    let mut gc = Heap::new();
    let err = gc.wait_host_process(999).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_destroying_invalid_process() {
    let mut gc = Heap::new();
    let err = gc.destroy_host_process(999).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_trying_exit_value_invalid_process() {
    let mut gc = Heap::new();
    let err = gc.try_host_process_exit_value(999).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_opening_invalid_zip() {
    let mut gc = Heap::new();
    let path = std::env::temp_dir().join("definitely_not_a_zip.zip");
    std::fs::write(&path, b"not a zip file content").unwrap();
    let err = gc.open_host_zip(&path).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/util/zip/ZipException")
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn should_return_error_when_accessing_invalid_zip_handle() {
    let gc = Heap::new();
    let err1 = gc.zip_entry_count(999).unwrap_err();
    assert!(
        matches!(err1, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
    let err2 = gc.zip_get_entry_info(999, "test").unwrap_err();
    assert!(
        matches!(err2, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
    let err3 = gc.zip_read_entry(999, "test").unwrap_err();
    assert!(
        matches!(err3, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}

#[test]
fn should_return_error_when_binding_invalid_socket_address() {
    let mut gc = Heap::new();
    let err = gc.bind_server_socket("invalid_address").unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/net/SocketException")
    );
}

#[test]
fn should_return_error_when_accepting_invalid_listener() {
    let mut gc = Heap::new();
    let err = gc.accept_connection(999).unwrap_err();
    assert!(
        matches!(err, VmError::JavaException { ref class_name } if class_name == "java/io/IOException")
    );
}
use super::*;

// ── Allocation ────────────────────────────────────────────────────────────

#[test]
fn allocate_and_get() {
    let mut heap = Heap::new();
    let r = heap.allocate("Point".to_string(), 2);
    assert_eq!(r & OLD_BIT, 0, "new object must be in young gen");
    let obj = heap.get(r).unwrap();
    assert_eq!(obj.class_name, "Point");
    assert_eq!(obj.fields.len(), 2);
    assert_eq!(obj.fields[0], Slot::Int(0));
}

#[test]
fn allocate_multiple() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("Point".to_string(), 2);
    let r1 = heap.allocate("Point".to_string(), 2);
    assert_eq!(r0, 0);
    assert_eq!(r1, 1);
    assert_eq!(heap.len(), 2);
}

#[test]
fn get_mut_sets_field() {
    let mut heap = Heap::new();
    let r = heap.allocate("Point".to_string(), 2);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
    assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
}

#[test]
fn invalid_ref_returns_error() {
    let heap = Heap::new();
    let err = heap.get(999).unwrap_err();
    assert!(matches!(err, VmError::InvalidRef { address: 999 }));
}

#[test]
fn get_on_invalid_ref_returns_error() {
    let heap = Heap::new();
    assert!(heap.get(0).is_err());
    assert!(heap.get(999).is_err());
}

#[test]
fn allocate_string_stores_value() {
    let mut heap = Heap::new();
    let r = heap.allocate_string("hello".to_string());
    let obj = heap.get(r).unwrap();
    assert_eq!(obj.class_name, "java/lang/String");
    assert_eq!(obj.string_value, Some("hello".to_string()));
    assert!(obj.fields.is_empty());
}

#[test]
fn heap_object_marked_defaults_false() {
    let mut heap = Heap::new();
    let r = heap.allocate("Foo".to_string(), 0);
    assert!(!heap.get(r).unwrap().marked);
}

#[test]
fn heap_object_age_defaults_zero() {
    let mut heap = Heap::new();
    let r = heap.allocate("Foo".to_string(), 0);
    assert_eq!(heap.get(r).unwrap().age, 0);
}

#[test]
fn heap_object_forward_defaults_none() {
    let mut heap = Heap::new();
    let r = heap.allocate("Foo".to_string(), 0);
    assert!(heap.get(r).unwrap().forward.is_none());
}

// ── GC triggers ───────────────────────────────────────────────────────────

#[test]
fn should_gc_triggers_at_2x_growth() {
    let mut heap = Heap::new();
    for i in 0..255 {
        heap.allocate(format!("C{i}"), 0);
        assert!(!heap.should_gc());
    }
    heap.allocate("C255".to_string(), 0);
    assert!(heap.should_gc());
}

// ── collect() compat shim ─────────────────────────────────────────────────

#[test]
fn collect_reclaims_unreachable() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("Keep".to_string(), 0);
    let _r1 = heap.allocate("Drop".to_string(), 0);
    let _r2 = heap.allocate("Drop".to_string(), 0);
    heap.collect(&[Slot::Reference(Some(r0))]);
    assert_eq!(heap.free_list_len(), 2);
    assert_eq!(heap.len(), 1);
}

#[test]
fn collect_preserves_reachable_chain() {
    let mut heap = Heap::new();
    let rc = heap.allocate("C".to_string(), 0);
    let rb = heap.allocate("B".to_string(), 1);
    heap.get_mut(rb).unwrap().fields[0] = Slot::Reference(Some(rc));
    let ra = heap.allocate("A".to_string(), 1);
    heap.get_mut(ra).unwrap().fields[0] = Slot::Reference(Some(rb));
    heap.collect(&[Slot::Reference(Some(ra))]);
    assert_eq!(heap.free_list_len(), 0);
    assert_eq!(heap.len(), 3);
}

#[test]
fn free_list_slot_reused_after_collect() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("Keep".to_string(), 0);
    let _r1 = heap.allocate("Drop".to_string(), 0);
    heap.collect(&[Slot::Reference(Some(r0))]);
    // After full collect, Keep was promoted to old gen.
    // Allocate a new object — it goes to young gen.
    let r2 = heap.allocate("New".to_string(), 0);
    // r2 should be a young-gen ref (OLD_BIT == 0).
    assert_eq!(r2 & OLD_BIT, 0);
    assert!(heap.get(r2).is_ok());
}

#[test]
fn get_on_swept_slot_returns_error() {
    let mut heap = Heap::new();
    let keep = heap.allocate("Keep".to_string(), 0);
    let drop_r = heap.allocate("Drop".to_string(), 0);
    heap.collect(&[Slot::Reference(Some(keep))]);
    // drop_r is now in the old-gen free list or absent.
    // Either way, get() on it must error.
    assert!(heap.get(drop_r).is_err() || heap.get(drop_r | OLD_BIT).is_err());
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn test_heap_with_capacity(cap: usize) -> Heap {
    let mut h = Heap::new();
    h.young_capacity = cap;
    h
}

fn make_old_obj(heap: &mut Heap) -> u64 {
    let obj = HeapObject {
        class_name: "OldObj".to_string(),
        fields: vec![Slot::Int(0)],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    };
    heap.old.push(Some(obj));
    (heap.old.len() as u64 - 1) | OLD_BIT
}

// ── GC trigger tests (Task 3) ──────────────────────────────────────────────

#[test]
fn should_minor_gc_fires_at_young_capacity() {
    let mut heap = Heap::new();
    heap.young_capacity = 4;
    // First 3 allocs should not trigger.
    for i in 0..3 {
        heap.allocate(format!("C{i}"), 0);
        assert!(
            !heap.should_minor_gc(),
            "should not fire before reaching capacity"
        );
    }
    // 4th alloc hits young_top == young_capacity → fires.
    heap.allocate("C3".to_string(), 0);
    assert!(heap.should_minor_gc());
}

#[test]
fn should_major_gc_fires_at_2x_old_live() {
    let mut heap = Heap::new();
    // Simulate post-GC state: 10 live old objects, alloc_since_gc reset to 0.
    // Use collect() on a fresh heap to set live_after_last_gc.
    // First, make 10 objects live through a collect.
    let roots: Vec<Slot> = (0..10)
        .map(|_| {
            let r = heap.allocate("O".to_string(), 0);
            Slot::Reference(Some(r))
        })
        .collect();
    heap.collect(&roots);
    // Now live_after_last_gc == 10 (all 10 in old gen after promotion).
    // threshold = max(10*2, 256) = 256. Must allocate 256 more.
    for i in 0..255 {
        heap.allocate(format!("X{i}"), 0);
        assert!(!heap.should_major_gc(), "should not fire at alloc {i}");
    }
    heap.allocate("X255".to_string(), 0);
    assert!(heap.should_major_gc());
}

// ── write_field tests (Task 4) ─────────────────────────────────────────────

#[test]
fn write_field_old_to_young_adds_to_remembered_set() {
    let mut heap = Heap::new();
    let old_ref = make_old_obj(&mut heap);
    let young_ref = heap.allocate("Young".to_string(), 0);
    heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
        .unwrap();
    let old_idx = (old_ref & !OLD_BIT) as usize;
    assert!(
        heap.remembered_set.contains(&old_idx),
        "old→young store must populate remembered_set"
    );
}

#[test]
fn write_field_young_to_young_does_not_add_to_remembered_set() {
    let mut heap = Heap::new();
    let r0 = heap.allocate("A".to_string(), 1);
    let r1 = heap.allocate("B".to_string(), 0);
    // r0 is young; store another young ref into it.
    heap.write_field(r0, 0, Slot::Reference(Some(r1))).unwrap();
    assert!(
        heap.remembered_set.is_empty(),
        "young→young store must NOT populate remembered_set"
    );
}

#[test]
fn write_field_old_to_old_does_not_add_to_remembered_set() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "A".to_string(),
        fields: vec![Slot::Int(0)],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "B".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let a_ref = OLD_BIT;
    let b_ref = 1u64 | OLD_BIT;
    heap.write_field(a_ref, 0, Slot::Reference(Some(b_ref)))
        .unwrap();
    assert!(
        heap.remembered_set.is_empty(),
        "old→old store must NOT populate remembered_set"
    );
}

// ── Minor GC unit tests (Task 6) ───────────────────────────────────────────

#[test]
fn minor_gc_copies_reachable_young_object() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("Keep".to_string(), 0);
    let r1 = heap.allocate("Drop".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    // r0 must have a forwarding pointer; r1 must not.
    assert!(
        heap.young[usize::try_from(r0).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .is_some()
    );
    assert!(
        heap.young[usize::try_from(r1).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .is_none()
    );
}

#[test]
fn minor_gc_forward_patches_root_slot() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("A".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    let mut slot = Slot::Reference(Some(r0));
    heap.apply_forward(&mut slot);
    // After forwarding, slot must point to the new location.
    let new_r = heap.young[usize::try_from(r0).unwrap()]
        .as_ref()
        .unwrap()
        .forward
        .unwrap();
    assert_eq!(slot, Slot::Reference(Some(new_r)));
}

#[test]
fn minor_gc_finish_swaps_to_space_into_young() {
    let mut heap = test_heap_with_capacity(8);
    let r0 = heap.allocate("A".to_string(), 0);
    let _r1 = heap.allocate("B".to_string(), 0);
    // Only r0 is a root → _r1 is dead.
    let roots = vec![Slot::Reference(Some(r0))];
    heap.minor_collect_prepare(&roots);
    heap.minor_collect_finish();
    // After finish: young has 1 live survivor.
    assert_eq!(heap.young.iter().filter(|s| s.is_some()).count(), 1);
    assert!(heap.to_space.is_empty());
    assert!(heap.remembered_set.is_empty());
}

#[test]
fn minor_gc_increments_age_on_survival() {
    let mut heap = test_heap_with_capacity(8);
    let r = heap.allocate("Survivor".to_string(), 0);
    let roots = vec![Slot::Reference(Some(r))];
    heap.minor_collect_prepare(&roots);
    // Locate the copy in to_space (new_ref from forward pointer).
    let new_r = heap.young[usize::try_from(r).unwrap()]
        .as_ref()
        .unwrap()
        .forward
        .unwrap();
    heap.minor_collect_finish();
    // After finish, young is former to_space. new_r has no OLD_BIT → young index.
    let survivor = heap.young[usize::try_from(new_r).unwrap()]
        .as_ref()
        .unwrap();
    assert_eq!(survivor.age, 1);
}

#[test]
fn minor_gc_promotes_at_promotion_age() {
    let mut heap = test_heap_with_capacity(64);
    // promotion_age = 1: an object with age >= 1 is promoted.
    // Round 0: age=0, check 0>=1 → false → to_space (age becomes 1), new_r is young.
    // Round 1: age=1, check 1>=1 → true  → old gen (OLD_BIT set).
    heap.promotion_age = 1;

    let mut current_r = heap.allocate("P".to_string(), 0);

    for round in 0..2u8 {
        let roots = vec![Slot::Reference(Some(current_r))];
        heap.minor_collect_prepare(&roots);
        let new_r = heap.young[usize::try_from(current_r).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        heap.minor_collect_finish();

        if round == 0 {
            // Still young after first survival (age becomes 1, not yet promoted).
            assert_eq!(new_r & OLD_BIT, 0, "should still be young after 1 survival");
            current_r = new_r;
        } else {
            // Promoted to old gen (OLD_BIT set) on second survival.
            assert_ne!(new_r & OLD_BIT, 0, "should be in old gen after 2 survivals");
            let obj = heap.get(new_r).unwrap();
            assert_eq!(obj.class_name, "P");
        }
    }
}

#[test]
fn remembered_set_root_survives_minor_gc() {
    let mut heap = test_heap_with_capacity(8);
    // Build: old-gen object with a field pointing to a young object.
    heap.old.push(Some(HeapObject {
        class_name: "Old".to_string(),
        fields: vec![Slot::Int(0)], // will be overwritten below
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let old_ref = OLD_BIT;
    let young_ref = heap.allocate("Young".to_string(), 0);
    // Wire old→young via write_field (populates remembered_set).
    heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
        .unwrap();

    // No stack roots — young object reachable only through remembered set.
    heap.minor_collect_prepare(&[]);
    assert!(
        heap.young[usize::try_from(young_ref).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .is_some(),
        "young object reachable via rem-set must be forwarded"
    );
    heap.minor_collect_finish();
    // Verify old-gen field was patched to the new young location.
    let new_field = heap.old[0].as_ref().unwrap().fields[0];
    match new_field {
        Slot::Reference(Some(r)) => {
            heap.get(r).expect("patched old→young field must be valid");
        }
        other => panic!("expected Reference, got {other:?}"),
    }
}

#[test]
fn apply_forward_is_no_op_on_non_references() {
    let heap = Heap::new();
    let mut slot = Slot::Int(42);
    heap.apply_forward(&mut slot);
    assert_eq!(slot, Slot::Int(42));
}

#[test]
fn apply_forward_is_no_op_on_null_ref() {
    let heap = Heap::new();
    let mut slot = Slot::Reference(None);
    heap.apply_forward(&mut slot);
    assert_eq!(slot, Slot::Reference(None));
}

#[test]
fn apply_forward_is_no_op_on_old_gen_ref() {
    let heap = Heap::new();
    let old_ref = OLD_BIT;
    let mut slot = Slot::Reference(Some(old_ref));
    heap.apply_forward(&mut slot);
    // No forwarding pointer in old gen → slot unchanged.
    assert_eq!(slot, Slot::Reference(Some(old_ref)));
}

// ── Major GC unit tests (Task 7) ───────────────────────────────────────────

#[test]
fn old_ref_has_old_bit() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "OldObj".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let old_ref = OLD_BIT;
    assert_ne!(old_ref & OLD_BIT, 0, "old ref must have OLD_BIT set");
    assert_eq!(heap.get(old_ref).unwrap().class_name, "OldObj");
}

#[test]
fn major_collect_reclaims_unreachable_old_objects() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "Keep".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "Drop".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let keep_ref = OLD_BIT;
    let drop_ref = 1u64 | OLD_BIT;
    let roots = vec![Slot::Reference(Some(keep_ref))];
    heap.major_collect(&roots);
    assert!(
        heap.get(keep_ref).is_ok(),
        "reachable old-gen object must survive"
    );
    assert!(
        heap.get(drop_ref).is_err(),
        "unreachable old-gen object must be swept"
    );
    assert_eq!(heap.old_free_list.len(), 1);
}

#[test]
fn major_collect_preserves_reachable_chain_in_old_gen() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "C".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "B".to_string(),
        fields: vec![Slot::Reference(Some(OLD_BIT))],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "A".to_string(),
        fields: vec![Slot::Reference(Some(1u64 | OLD_BIT))],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let a_ref = 2u64 | OLD_BIT;
    let roots = vec![Slot::Reference(Some(a_ref))];
    heap.major_collect(&roots);
    assert_eq!(heap.old_live_count(), 3);
    assert_eq!(heap.old_free_list.len(), 0);
}

#[test]
fn major_collect_reuses_freed_slot() {
    let mut heap = Heap::new();
    heap.old.push(Some(HeapObject {
        class_name: "Keep".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "Drop".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let keep_ref = OLD_BIT;
    heap.major_collect(&[Slot::Reference(Some(keep_ref))]);
    // old_free_list has raw index 1 (no OLD_BIT).
    assert!(heap.old_free_list.contains(&1u64));
}

#[test]
fn collect_compat_shim_collects_full_heap() {
    let mut heap = test_heap_with_capacity(512);
    // promotion_age = 0: age >= 0 is always true, so any survivor promotes
    // on the first minor GC.  This lets the compat collect() shim produce a
    // single old-gen object in one pass.
    heap.promotion_age = 0;
    let keep = heap.allocate("Keep".to_string(), 0);
    let _drop1 = heap.allocate("Drop1".to_string(), 0);
    let _drop2 = heap.allocate("Drop2".to_string(), 0);
    let roots = vec![Slot::Reference(Some(keep))];
    heap.collect(&roots);
    // After full collect: 1 live object promoted to old gen; 2 dropped.
    assert_eq!(heap.old_live_count(), 1);
    assert_eq!(heap.len(), 1);
}

// ── allocate_string coverage ───────────────────────────────────────────────

#[test]
fn allocate_string_increments_live_count() {
    let mut heap = Heap::new();
    assert_eq!(heap.len(), 0);
    heap.allocate_string("a".to_string());
    assert_eq!(heap.len(), 1);
    heap.allocate_string("b".to_string());
    assert_eq!(heap.len(), 2);
}

#[test]
fn allocate_string_returns_distinct_refs() {
    let mut heap = Heap::new();
    let r0 = heap.allocate_string("hello".to_string());
    let r1 = heap.allocate_string("world".to_string());
    assert_ne!(r0, r1);
    assert_eq!(
        heap.get(r0).unwrap().string_value,
        Some("hello".to_string())
    );
    assert_eq!(
        heap.get(r1).unwrap().string_value,
        Some("world".to_string())
    );
}

#[test]
fn allocate_string_increments_alloc_since_gc() {
    let mut heap = Heap::new();
    for i in 0..255 {
        heap.allocate_string(format!("s{i}"));
        assert!(!heap.should_gc());
    }
    heap.allocate_string("s255".to_string());
    assert!(heap.should_gc());
}

// ── promote_to_old free-list path ─────────────────────────────────────────

#[test]
fn promote_to_old_free_list_path_sets_old_bit() {
    let mut heap = test_heap_with_capacity(64);
    heap.promotion_age = 0; // promote on first minor GC

    // Round 1: promote an object via push path → old[0]
    let r0 = heap.allocate("TempOld".to_string(), 0);
    heap.minor_collect_prepare(&[Slot::Reference(Some(r0))]);
    heap.minor_collect_finish();
    // old[0] holds TempOld; major GC with no roots frees it → raw idx 0 → free list
    heap.major_collect(&[]);
    assert!(
        !heap.old_free_list.is_empty(),
        "free list must be non-empty after sweep"
    );

    // Round 2: promote via free-list path (raw_idx=0 from old_free_list)
    let r1 = heap.allocate("NewObj".to_string(), 0);
    heap.minor_collect_prepare(&[Slot::Reference(Some(r1))]);
    let new_r1 = heap.young[usize::try_from(r1).unwrap()]
        .as_ref()
        .unwrap()
        .forward
        .unwrap();
    heap.minor_collect_finish();

    assert_ne!(
        new_r1 & OLD_BIT,
        0,
        "promoted object (via free-list) must have OLD_BIT set"
    );
    assert!(heap.get(new_r1).is_ok());
    assert_eq!(heap.get(new_r1).unwrap().class_name, "NewObj");
}

// ── is_empty ──────────────────────────────────────────────────────────────

#[test]
fn is_empty_on_fresh_heap() {
    let heap = Heap::new();
    assert!(heap.is_empty());
}

#[test]
fn is_empty_false_after_allocation() {
    let mut heap = Heap::new();
    heap.allocate("X".to_string(), 0);
    assert!(!heap.is_empty());
}

// ── should_major_gc 2× multiplier ────────────────────────────────────────

#[test]
fn should_major_gc_uses_2x_threshold_not_add_or_div() {
    let mut heap = test_heap_with_capacity(512);
    heap.promotion_age = 0;
    // Populate old gen with 200 live objects via collect → live_after_last_gc=200.
    let roots: Vec<Slot> = (0..200)
        .map(|_| Slot::Reference(Some(heap.allocate("O".to_string(), 0))))
        .collect();
    heap.collect(&roots);
    // threshold = max(200*2, 256) = 400. Allocate 300 → should NOT fire.
    for _ in 0..300 {
        heap.allocate("X".to_string(), 0);
    }
    assert!(
        !heap.should_major_gc(),
        "should not fire at 300 allocs (threshold 400 with 200 live)"
    );
    // Allocate 100 more → alloc_since_gc=400 ≥ threshold=400 → should fire.
    for _ in 0..100 {
        heap.allocate("X".to_string(), 0);
    }
    assert!(
        heap.should_major_gc(),
        "should fire at 400 allocs (threshold 400 with 200 live)"
    );
}

// ── has_pending_forwards ──────────────────────────────────────────────────

#[test]
fn has_pending_forwards_false_before_gc() {
    let heap = Heap::new();
    assert!(!heap.has_pending_forwards());
}

#[test]
fn has_pending_forwards_true_after_prepare() {
    let mut heap = test_heap_with_capacity(8);
    let r = heap.allocate("A".to_string(), 0);
    heap.minor_collect_prepare(&[Slot::Reference(Some(r))]);
    assert!(heap.has_pending_forwards());
}

// ── minor GC patches old-gen fields when survivor index changes ──────────

#[test]
fn minor_gc_patches_old_gen_field_when_young_index_changes() {
    let mut heap = test_heap_with_capacity(8);
    // Allocate two young objects: young[0] will die, young[1] will survive.
    let dead = heap.allocate("Dead".to_string(), 0);
    let survive = heap.allocate("Survive".to_string(), 0);
    assert_eq!(dead, 0);
    assert_eq!(survive, 1, "survive must be at young index 1 for this test");

    // Create old object with field pointing to young[1]; write_field populates remembered_set.
    let old_ref = make_old_obj(&mut heap); // old[0], fields[0] = Int(0)
    heap.write_field(old_ref, 0, Slot::Reference(Some(survive)))
        .unwrap();

    // Minor GC: no stack roots. young[1] survives via remembered set → forwarded to to_space[0].
    heap.minor_collect_prepare(&[]);
    heap.minor_collect_finish();

    // After GC: young = to_space = [Some(Survive)]. "Survive" is now at index 0.
    // Old field must be patched from 1 → 0.
    let field = heap.get(old_ref).unwrap().fields[0];
    match field {
        Slot::Reference(Some(r)) => {
            assert_eq!(r & OLD_BIT, 0, "patched ref must be young");
            assert!(
                heap.get(r).is_ok(),
                "patched old→young field must be accessible"
            );
            assert_eq!(heap.get(r).unwrap().class_name, "Survive");
        }
        other => panic!("expected Reference, got {other:?}"),
    }
}

#[test]
fn minor_gc_patches_young_survivor_field_when_child_index_changes() {
    let mut heap = test_heap_with_capacity(8);
    let _dead = heap.allocate("Dead".to_string(), 0);
    let parent = heap.allocate("Parent".to_string(), 1);
    let child = heap.allocate("Child".to_string(), 0);
    heap.write_field(parent, 0, Slot::Reference(Some(child)))
        .unwrap();

    heap.minor_collect_prepare(&[Slot::Reference(Some(parent))]);
    let mut parent_slot = Slot::Reference(Some(parent));
    heap.apply_forward(&mut parent_slot);
    let Slot::Reference(Some(new_parent)) = parent_slot else {
        panic!("expected forwarded parent ref");
    };
    heap.minor_collect_finish();

    let Slot::Reference(Some(patched_child)) = heap.get(new_parent).unwrap().fields[0] else {
        panic!("expected forwarded child ref");
    };
    assert_eq!(
        patched_child, 1,
        "child should move from young[2] to young[1]"
    );
    assert_eq!(heap.get(patched_child).unwrap().class_name, "Child");
}

#[test]
fn promoted_old_object_keeps_remembered_set_for_next_minor_gc() {
    let mut heap = test_heap_with_capacity(8);
    heap.promotion_age = 0;

    let parent = heap.allocate("Parent".to_string(), 1);
    let child = heap.allocate("Child".to_string(), 0);
    heap.write_field(parent, 0, Slot::Reference(Some(child)))
        .unwrap();

    heap.minor_collect_prepare(&[Slot::Reference(Some(parent))]);
    let mut parent_slot = Slot::Reference(Some(parent));
    heap.apply_forward(&mut parent_slot);
    let Slot::Reference(Some(promoted_parent)) = parent_slot else {
        panic!("expected promoted parent ref");
    };
    assert_ne!(
        promoted_parent & OLD_BIT,
        0,
        "parent should promote on first survival"
    );
    heap.minor_collect_finish();

    let _dead = heap.allocate("Dead".to_string(), 0);
    heap.minor_collect_prepare(&[]);
    heap.minor_collect_finish();

    let Slot::Reference(Some(still_live_child)) = heap.get(promoted_parent).unwrap().fields[0]
    else {
        panic!("expected promoted parent to keep child ref");
    };
    assert_eq!(heap.get(still_live_child).unwrap().class_name, "Child");
}

// ── minor_collect_finish live_count ───────────────────────────────────────

#[test]
fn minor_collect_finish_live_count_sums_generations() {
    let mut heap = test_heap_with_capacity(8);
    // One old object (directly pushed, bypasses live_count).
    let _old_ref = make_old_obj(&mut heap);
    // One young object that survives the minor GC.
    let young_r = heap.allocate("Y".to_string(), 0);
    heap.minor_collect_prepare(&[Slot::Reference(Some(young_r))]);
    heap.minor_collect_finish();
    // minor_collect_finish recalculates live_count = young_live + old_live = 1 + 1 = 2.
    assert_eq!(heap.len(), 2);
}

// ── major_collect: young refs in roots/children must be ignored ──────────

#[test]
fn major_gc_ignores_young_refs_in_roots() {
    let mut heap = Heap::new();
    // Two old objects: old[0] should die, old[1] should survive.
    heap.old.push(Some(HeapObject {
        class_name: "ShouldDie".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    heap.old.push(Some(HeapObject {
        class_name: "ShouldLive".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let die_ref = OLD_BIT;
    let live_ref = 1u64 | OLD_BIT;
    // Young object at index 0 — same raw index as old[0].
    let young_r = heap.allocate("Young".to_string(), 0);
    assert_eq!(young_r, 0);
    // Roots: live_ref (old) + young_r (young). Young ref must NOT mark old[0].
    heap.major_collect(&[
        Slot::Reference(Some(live_ref)),
        Slot::Reference(Some(young_r)),
    ]);
    assert!(
        heap.get(die_ref).is_err(),
        "unreachable old object must be swept even when young ref shares raw index"
    );
    assert!(
        heap.get(live_ref).is_ok(),
        "reachable old object must survive"
    );
}

#[test]
fn major_gc_does_not_follow_young_refs_as_old_gen_children() {
    let mut heap = Heap::new();
    // old[0] = "ShouldDie" (unreachable); raw idx 0 matches young[0].
    heap.old.push(Some(HeapObject {
        class_name: "ShouldDie".to_string(),
        fields: vec![],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    // Allocate young[0] so raw idx 0 exists in young gen too.
    let young_r = heap.allocate("Young".to_string(), 0);
    assert_eq!(young_r, 0);
    // old[1] = "Parent" with a field pointing to young[0].
    heap.old.push(Some(HeapObject {
        class_name: "Parent".to_string(),
        fields: vec![Slot::Reference(Some(young_r))],
        string_value: None,
        marked: false,
        age: 0,
        forward: None,
    }));
    let die_ref = OLD_BIT;
    let parent_ref = 1u64 | OLD_BIT;
    // Root is only "Parent". The young ref in Parent's fields must not mark old[0].
    heap.major_collect(&[Slot::Reference(Some(parent_ref))]);
    assert!(
        heap.get(die_ref).is_err(),
        "old[0] must be swept: the young ref in Parent's fields must not mark it"
    );
    assert!(heap.get(parent_ref).is_ok(), "Parent must survive");
}

// ── Performance Tests ──────────────────────────────────────────────────────

#[test]
fn minor_gc_avoids_intermediate_allocs() {
    let code = std::fs::read_to_string("src/lib.rs").unwrap();
    // find the start of the minor_collect_prepare function
    let start = code.find("pub fn minor_collect_prepare").unwrap();
    let end = code[start..].find("pub fn minor_collect_finish").unwrap();
    let function_body = &code[start..start + end];

    assert!(
        !function_body.contains("let young_refs: Vec<usize> ="),
        "minor_collect_prepare should not collect into an intermediate vector for young_refs"
    );
    assert!(
        !function_body.contains("let children: Vec<usize> ="),
        "minor_collect_prepare should not collect into an intermediate vector for children"
    );
}

// ── TCP socket tests (Task 1) ──────────────────────────────────────────────

#[test]
fn bind_server_socket_returns_valid_id() {
    let mut heap = Heap::new();
    let id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
    assert!(id > 0);
}

#[test]
fn bind_server_socket_addr_in_use() {
    let mut heap = Heap::new();
    let id = heap
        .bind_server_socket("127.0.0.1:0")
        .expect("first bind failed");
    let port = heap.server_socket_local_port(id).expect("port failed");
    let err = heap
        .bind_server_socket(&format!("127.0.0.1:{port}"))
        .unwrap_err();
    assert!(matches!(
        err,
        duke_runtime::VmError::JavaException { ref class_name }
        if class_name == "java/net/BindException"
    ));
}

#[test]
fn connect_socket_refused() {
    // On Windows, WSAECONNREFUSED may not map to ErrorKind::ConnectionRefused in all
    // Rust versions. Accept either ConnectException or SocketException so the test
    // passes on all platforms while still verifying no panic occurs.
    // Bind to get a port, then drop the listener so nothing listens.
    let mut heap = Heap::new();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let err = heap
        .connect_socket(&format!("127.0.0.1:{port}"))
        .unwrap_err();
    assert!(matches!(
        err,
        duke_runtime::VmError::JavaException { ref class_name }
        if class_name == "java/net/ConnectException"
            || class_name == "java/net/SocketException"
    ));
}

#[test]
fn server_socket_local_port() {
    let mut heap = Heap::new();
    let id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
    let port = heap.server_socket_local_port(id).expect("port failed");
    assert!(port > 0);
}

#[test]
fn accept_and_read_roundtrip() {
    let mut heap = Heap::new();
    let server_id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
    let port = heap
        .server_socket_local_port(server_id)
        .expect("port failed");
    let handle = std::thread::spawn(move || {
        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        std::io::Write::write_all(&mut stream, &[42]).unwrap();
    });
    let (reader_id, _writer_id) = heap.accept_connection(server_id).expect("accept failed");
    let byte = heap.read_host_file_byte(reader_id).expect("read failed");
    assert_eq!(byte, 42);
    handle.join().unwrap();
}

#[test]
fn connect_and_write_roundtrip() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1];
        std::io::Read::read_exact(&mut stream, &mut buf).unwrap();
        assert_eq!(buf[0], 99);
    });
    let mut heap = Heap::new();
    let (_reader_id, writer_id) = heap
        .connect_socket(&format!("127.0.0.1:{port}"))
        .expect("connect failed");
    heap.write_host_file_byte(writer_id, 99)
        .expect("write failed");
    // Drop the writer so the listener's read_exact completes.
    heap.close_host_file(writer_id);
    handle.join().unwrap();
}

#[test]
fn close_then_read_returns_io_exception() {
    let mut heap = Heap::new();
    let server_id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
    let port = heap
        .server_socket_local_port(server_id)
        .expect("port failed");
    let handle = std::thread::spawn(move || {
        let _stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    });
    let (reader_id, _writer_id) = heap.accept_connection(server_id).expect("accept failed");
    handle.join().unwrap();
    heap.close_host_file(reader_id);
    let err = heap.read_host_file_byte(reader_id).unwrap_err();
    assert!(matches!(
        err,
        duke_runtime::VmError::JavaException { ref class_name }
        if class_name == "java/io/IOException"
    ));
}
