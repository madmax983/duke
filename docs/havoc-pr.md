# 👺 Havoc: Thread.join() self-join panic

🧨 **The Trigger:** Java code calling `this.join()` inside a thread's `run()` method (or `Thread.currentThread().join()`) causes the interpreter to look up its own `std::thread::JoinHandle` and call `.join()` on it. This causes a Rust panic (`thread attempted to join itself`), which poisons the global JVM mutexes and crashes the entire VM.

📉 **The Stack Trace:**
```
thread 'tests::threading_havoc_self_join_panics' panicked at library/std/src/thread/mod.rs:1680:24:
thread attempted to join itself
```

🧪 **Reproduction:** Execute a class with `public void run() { this.join(); }` or run `cargo test -p duke-interpreter threading_havoc_self_join_panics`.

😈 **Comment:** You assumed threads would only join other threads. You forgot Java threads can join themselves. Now your JVM is dead.
