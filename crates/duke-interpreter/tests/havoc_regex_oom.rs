use duke_interpreter::ClassRegistry;

#[test]
fn test_havoc_regex_split_oom() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    duke_interpreter::bootstrap_stdlib(&mut registry, &mut heap);

    let handler = registry.natives().get(
        "java/lang/String",
        "split",
        "(Ljava/lang/String;)[Ljava/lang/String;",
    );
    if let Some(h) = handler {
        let this_str = heap.allocate_string("hello".to_string());

        let delim = "(".repeat(10_000_000);
        let delim_str = heap.allocate_string(delim);

        let args = vec![
            duke_runtime::Slot::Reference(Some(this_str)),
            duke_runtime::Slot::Reference(Some(delim_str)),
        ];
        let mut out = Vec::new();
        let mut control = duke_interpreter::NativeControl::default();

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            h(&args, &mut heap, &mut out, &mut control)
        }));

        let handler_result =
            res.expect("Handler panicked! `native_string_split` unwrap must be fixed.");
        assert!(
            handler_result.is_err(),
            "Expected PatternSyntaxException but got {handler_result:?}"
        );
    } else {
        panic!("Missing handler");
    }
}

#[test]
fn test_havoc_regex_split_limit_oom() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    duke_interpreter::bootstrap_stdlib(&mut registry, &mut heap);

    let handler = registry.natives().get(
        "java/lang/String",
        "split",
        "(Ljava/lang/String;I)[Ljava/lang/String;",
    );
    if let Some(h) = handler {
        let this_str = heap.allocate_string("hello".to_string());

        let delim = "a".repeat(10_000_000);
        let delim_str = heap.allocate_string(delim);

        let args = vec![
            duke_runtime::Slot::Reference(Some(this_str)),
            duke_runtime::Slot::Reference(Some(delim_str)),
            duke_runtime::Slot::Int(10),
        ];
        let mut out = Vec::new();
        let mut control = duke_interpreter::NativeControl::default();

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            h(&args, &mut heap, &mut out, &mut control)
        }));

        let handler_result =
            res.expect("Handler panicked! `native_string_split_limit` unwrap must be fixed.");
        assert!(
            handler_result.is_err(),
            "Expected PatternSyntaxException but got {handler_result:?}"
        );
    } else {
        panic!("Missing handler");
    }
}
