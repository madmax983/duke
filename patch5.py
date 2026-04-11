import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

# Let's add a println inside the loop! Maybe the thread ID is wrong?
content_new = content.replace(
"""fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    loop {""",
"""fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    println!("thread joining {}", thread_id);
    loop {""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
