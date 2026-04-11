# 👺 Havoc: Deadlock in `wait_for_all_java_threads`

* 🧨 **The Trigger:** A Java thread calling `Thread.join()` on another thread while the JVM is shutting down (`wait_for_all_java_threads`).
* 📉 **The Stack Trace:** (Deadlocked. The process spins 100% CPU on `thread::yield_now()` indefinitely).
* 🧪 **Reproduction:** Run `cargo test --test havoc_threading_deadlock` which demonstrates that `drain()` steals all handles, preventing `join_java_thread` from finding its handle and spinning infinitely.
* 😈 **Comment:** You assumed `drain()` was safe because no new threads would spawn. But existing threads might still try to join each other! Now they loop forever looking for a handle you stole.
