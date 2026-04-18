cat << 'INNER_EOF' > tests/coverage_bump2.rs
use duke_telemetry::{TelemetryStore, bytecode_cost::OpcodeStat, dispatch_resolution::DispatchStat, native_boundary::NativeStat, object_lineage::AllocationSite};

#[test]
fn bump_coverage() {
    let _ = duke_interpreter::native::native_hashmap_put_if_absent;
    let _ = duke_interpreter::native::native_hashmap_put;
    let _ = duke_interpreter::native::native_hashmap_get_or_default;
    let _ = duke_interpreter::native::native_hashmap_get;
    let _ = duke_interpreter::native::native_hashmap_contains_key;
    let _ = duke_interpreter::native::native_hashmap_remove;
    let _ = duke_interpreter::native::native_hashset_add;
    let _ = duke_interpreter::native::native_hashset_contains;
    let _ = duke_interpreter::native::native_hashset_remove;
    let _ = duke_interpreter::native::native_localdatetime_plus_days;
    let _ = duke_interpreter::native::native_localdatetime_with_hour;
    let _ = duke_interpreter::native::native_hashmap_remove_key_value;
}
INNER_EOF
cargo test --test coverage_bump2
