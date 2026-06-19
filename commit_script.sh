git add crates/duke-interpreter/src/native.rs
git commit -m "⚡ Bolt: Remove unnecessary heap allocations in native methods" -m "💡 What: Replaced vector allocations by using standard loops over \`heap.get(...)\`.
🎯 Why: Several methods created full clones of \`fields\` just to read elements, which required allocations.
📊 Impact: Reduced heap allocations on several core method pathways in interpreter's natives.
🔬 Measurement: Run \`cargo bench\`."
