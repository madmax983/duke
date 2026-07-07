use duke_gc::Heap;
use duke_runtime::Error;

#[test]
fn test_host_io_error_handling() {
    let mut heap = Heap::new();

    let wait_res = heap.wait_host_process(999);
    assert!(
        matches!(wait_res, Err(Error::JavaException { ref class_name }) if class_name == "java/io/IOException")
    );
}
