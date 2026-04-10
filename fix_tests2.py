import re

with open('crates/duke-gc/src/host.rs', 'r') as f:
    text = f.read()

# Add #[test]\nfn test_host_file_operations() with `let mut heap = Heap::new(); heap.host.xxx`
# The problem is `host.rs` only has ~86% coverage, we need to test the enum variants or something.
# We will just write tests for every method on HostManager directly so we don't rely on `heap.host`.

test_all_host_manager = """
#[cfg(test)]
mod tests2 {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_host_manager_debug() {
        let host = HostManager::new();
        let debug_str = format!("{:?}", host);
        assert!(debug_str.contains("HostManager"));
    }

    #[test]
    fn test_spawned_process_ids_debug() {
        let ids = SpawnedProcessIds { process_id: 1, stdin_id: 2, stdout_id: 3, stderr_id: 4 };
        let debug_str = format!("{:?}", ids);
        assert!(debug_str.contains("SpawnedProcessIds"));
    }

    #[test]
    fn test_host_file_handle_debug() {
        let handle = HostFileHandle::ByteBuffer(std::io::Cursor::new(vec![]));
        let debug_str = format!("{:?}", handle);
        assert!(debug_str.contains("ByteBuffer"));
    }

    #[test]
    fn test_host_manager_default_impl() {
        let host = HostManager::default();
        assert_eq!(host.next_host_file_id, 1);
    }
}
"""

with open('crates/duke-gc/src/host.rs', 'a') as f:
    f.write(test_all_host_manager)
