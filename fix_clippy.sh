sed -i 's/pub fn run_execution(/#[allow(clippy::large_stack_frames)]\npub fn run_execution(/' crates/duke-interpreter/src/execution.rs
