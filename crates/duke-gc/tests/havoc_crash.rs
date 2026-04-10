use duke_gc::Heap;

#[test]
fn havoc_get_u64_max() {
    let heap = Heap::new();
    let r = u64::MAX;
    // this will no longer crash but return InvalidRef
    let res = heap.get(r);
    assert!(res.is_err());
}

#[test]
fn havoc_get_u64_max_old_bit() {
    let heap = Heap::new();
    let r = u64::MAX | duke_gc::OLD_BIT;
    let res = heap.get(r);
    assert!(res.is_err());
}

#[test]
fn havoc_get_young_out_of_bounds() {
    let heap = Heap::new();
    let r = 9999;
    let res = heap.get(r);
    assert!(res.is_err());
}

#[test]
fn havoc_get_mut_u64_max() {
    let mut heap = Heap::new();
    let r = u64::MAX;
    let res = heap.get_mut(r);
    assert!(res.is_err());
}

#[test]
fn havoc_get_mut_u64_max_old_bit() {
    let mut heap = Heap::new();
    let r = u64::MAX | duke_gc::OLD_BIT;
    let res = heap.get_mut(r);
    assert!(res.is_err());
}
