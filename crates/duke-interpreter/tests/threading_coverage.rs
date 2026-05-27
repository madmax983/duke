use duke_interpreter::ThreadRuntime;

#[test]
fn test_mark_finished_by_java_ref_returns_false() {
    let mut runtime = ThreadRuntime::new();
    assert!(!runtime.mark_finished_by_java_ref(1));
}

#[test]
fn test_mark_finished_returns_false() {
    let mut runtime = ThreadRuntime::new();
    assert!(!runtime.mark_finished(1));
}
