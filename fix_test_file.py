import re
with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    content = f.read()

# Add back the test using crate::* instead of super::*
if "threading_havoc_join_self_deadlocks_instead_of_panic" not in content:
    content = content.replace("mod fuzz;", """mod fuzz;

#[cfg(test)]
mod havoc_thread_self_join_test {
    use crate::*;

    #[test]
    fn threading_havoc_join_self_deadlocks_instead_of_panic() {
        let handle = std::thread::spawn(|| {
            let result = std::panic::catch_unwind(|| {
                crate::tests::run_bootstrap_with_output("JoinSelf.class", "main", "([Ljava/lang/String;)V")
            });
            result
        });

        let timeout = std::time::Duration::from_millis(500);
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if handle.is_finished() {
                let res = handle.join().unwrap();
                assert!(res.is_ok(), "Thread panicked!");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
""")
with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(content)
