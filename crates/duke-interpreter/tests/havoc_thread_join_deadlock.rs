use duke_interpreter::execute_class_to_completion;
use duke_interpreter::registry::ClassRegistry;
use duke_loader::DirectoryLoader;
use duke_runtime::Slot;

#[test]
fn havoc_thread_join_deadlock() {
    let mut registry = ClassRegistry::new();
    let loader = DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
    );
    let mut heap = duke_gc::Heap::new();
    let mut stdout = Vec::<u8>::new();

    duke_interpreter::bootstrap_stdlib(&mut registry, &mut heap);

    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut stdout,
        "ThreadSelfJoin2",
        "main",
        "([Ljava/lang/String;)V",
        &[Slot::Reference(None)],
    );

    assert!(
        result.is_err(),
        "Expected IllegalThreadState error, got {result:?}",
    );
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        duke_runtime::VmError::IllegalThreadState { .. }
    ));
}
