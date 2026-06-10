## 2023-10-25 - [Fix thread interruption memory leak]
**The Trigger:** "Thread `interrupt_host_thread` race condition caused `interrupted_host_threads` to leak `ThreadId`s."
**The Stack Trace:** "(Loom test `test_interrupted_threads_leak` failed: Memory leak detected in interrupted_host_threads!)"
**Reproduction:** "Run `cargo test --test havoc_interrupted_threads_leak`."
**Comment:** "You assumed `ThreadId`s would magically clean themselves up. You were wrong."

## 2023-10-25 - [Fix thread interruption memory leak]
**The Trigger:** "Thread `interrupt_host_thread` race condition caused `interrupted_host_threads` to leak `ThreadId`s."
**The Stack Trace:** "(Loom test `test_interrupted_threads_leak` failed: Memory leak detected in interrupted_host_threads!)"
**Reproduction:** "Run `cargo test --test havoc_interrupted_threads_leak`."
**Comment:** "You assumed `ThreadId`s would magically clean themselves up. You were wrong."

## 2023-10-25 - [Fix thread interruption memory leak]
**The Trigger:** "Thread `interrupt_host_thread` race condition caused `interrupted_host_threads` to leak `ThreadId`s."
**The Stack Trace:** "(Loom test `test_interrupted_threads_leak` failed: Memory leak detected in interrupted_host_threads!)"
**Reproduction:** "Run `cargo test --test havoc_interrupted_threads_leak`."
**Comment:** "You assumed `ThreadId`s would magically clean themselves up. You were wrong."
