Wait, FxHashSet is not in workspace dependencies. And I can't add dependencies without asking.
"Swap default generic HashMap for FxHashMap (fast-hash) on integer keys" -> memory note says "if available".
Let's see what I can do.

Let's check `crates/duke-bytecode/src/basic_block.rs`. `instructions[current_start_idx..i].to_vec()` creates a new `Vec` for every basic block. We can pre-allocate it with `Vec::with_capacity(i - current_start_idx)`. But `to_vec()` does exactly this because it knows the length of the slice! So `to_vec()` is optimal.

Wait, `construct_blocks` has:
```rust
fn construct_blocks(
    instructions: &[(usize, Instruction)],
    leaders: &BTreeSet<usize>,
) -> Vec<BasicBlock> {
    let mut blocks = Vec::with_capacity(leaders.len());
```
`BTreeSet<usize>` in `find_leaders` is used to collect leaders. A `BTreeSet` allocates a node on the heap for every element! For just collecting `usize` (the program counters), a `Vec<usize>` that is sorted and deduplicated, or simply a `std::collections::HashSet` would be much faster. However, because it checks `leaders.contains(pc)` for every instruction `(i, (pc, _))` which are ALREADY SORTED, we could just use a `Vec<usize>` and a linear scan or binary search. But actually, `BTreeSet` isn't bad if it's small. Wait, a bitset or vector is faster. Or `HashSet`.

Another thing: `crates/duke-bytecode/src/instruction.rs`: `switch_targets` was already optimized to return `Iterator`! Wait, no, `control_flow_targets` in `instruction.rs` returns `Vec<usize>` and allocates it every time:
```rust
    pub fn control_flow_targets(&self, current_pc: usize, next_pc: Option<usize>) -> Vec<usize> {
        if self.is_return() {
            return Vec::new();
        }
        let mut targets = Vec::with_capacity(2);
        if let Some(offset) = self.unconditional_jump_target() {
            targets.push((current_pc as isize + offset) as usize);
            // ...
```
It returns a `Vec`.

What about `crates/duke-loader/src/zip.rs` `Vec::new()` without capacity?
```rust
    fn find_resources(&self, name: &str) -> Result<Vec<Vec<u8>>> {
        let mut resources = Vec::new();
        for entry in &self.classpath {
            resources.extend(entry.find_resources(name)?);
        }
        Ok(resources)
    }
```

What about `crates/duke-classfile/src/lib.rs` line 58?
```rust
    fn minimal_class_bytes() -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();

        // magic
        v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
...
```
This is a test function.

What about `crates/duke-loader/src/zip.rs` line 911?
`let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());` -> can be `Vec::with_capacity(something)`.

Let's look at `crates/duke-gc/src/lib.rs`:
```rust
        Self {
            heap: vec![0; initial_size.0],
            young: Vec::new(),
            young_size: 0,
            to_space: Vec::new(),
            to_space_size: 0,
            old: Vec::new(),
            old_size: 0,
            old_free_list: Vec::new(),
            next_ref: 1,
            // ...
        }
```

Wait, what about `crates/duke-interpreter/src/native.rs` line 21496?
```rust
                    if let AttributeData::LineNumberTable(entries) = &attr.data {
                        Some(
                            entries
                                .iter()
                                .map(|entry| (entry.start_pc, entry.line_number))
                                .collect::<Vec<_>>(),
                        )
```

Look at `crates/duke-bytecode/src/instruction.rs`:
```rust
    pub fn control_flow_targets(&self, current_pc: usize, next_pc: Option<usize>) -> Vec<usize> {
        if self.is_return() {
            return Vec::new();
        }
```

Wait, `control_flow_targets` returning a `Vec` is expensive if called for every instruction in `build_basic_blocks`!
Actually, `SmallVec` or `TinyVec` is requested! "Use SmallVec or TinyVec to keep small lists on the stack." But we'd need to add the dependency...
"Never do: ... Adding any new crates to Cargo.toml without instruction ... Ask first"

Is there an allocation we can eliminate completely?
Look at `duke-bytecode/src/basic_block.rs`:
```rust
fn find_leaders(instructions: &[(usize, Instruction)]) -> BTreeSet<usize> {
    let mut leaders = BTreeSet::new();
    leaders.insert(instructions[0].0);

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let is_branch_or_return = instr.is_conditional_branch()
            || instr.is_unconditional_jump()
            || instr.is_switch()
            || instr.is_return();

        // Target of any jump or branch is a leader
        for target in instr.control_flow_targets(*pc, None) {
            leaders.insert(target);
        }

        // The instruction immediately following any jump, branch, or return is a leader
        if is_branch_or_return && i + 1 < instructions.len() {
            leaders.insert(instructions[i + 1].0);
        }
    }
    leaders
}
```
`instr.control_flow_targets(*pc, None)` allocates a `Vec<usize>` for EVERY instruction to return its targets! We can add a method `control_flow_targets_iter` to `Instruction` or we can just do the logic inline, or pass a `&mut Vec<usize>` to `control_flow_targets` to reuse the allocation!
Wait! Even simpler: `control_flow_targets` is only returning up to 2 items usually (except for switch which can return many).
But we can avoid `BTreeSet`!
Wait, `leaders` is a `BTreeSet<usize>`.
If we change `leaders` to a `Vec<usize>`, we can `leaders.push(target)`, and at the end do `leaders.sort_unstable(); leaders.dedup();`. Then in `construct_blocks`, instead of `leaders.contains(pc)`, we can just use a two-pointer approach or `leaders.binary_search(&pc).is_ok()`!
Since `leaders.contains(pc)` is called for every PC in increasing order, we can just track the `next_leader_idx` and check `pc == leaders[next_leader_idx]`!
This avoids ALL the heap allocations of `BTreeSet` nodes!

Let's check the size of `leaders`. It can be hundreds for large methods.
`BTreeSet` allocates a node per element.
If we change `BTreeSet<usize>` to `Vec<usize>`:
```rust
fn find_leaders(instructions: &[(usize, Instruction)]) -> Vec<usize> {
    let mut leaders = Vec::with_capacity(instructions.len() / 4); // Guess capacity
    leaders.push(instructions[0].0);

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let is_branch_or_return = instr.is_conditional_branch()
            || instr.is_unconditional_jump()
            || instr.is_switch()
            || instr.is_return();

        for target in instr.control_flow_targets(*pc, None) {
            leaders.push(target);
        }

        if is_branch_or_return && i + 1 < instructions.len() {
            leaders.push(instructions[i + 1].0);
        }
    }

    leaders.sort_unstable();
    leaders.dedup();
    leaders
}
```
And `construct_blocks`:
```rust
fn construct_blocks(
    instructions: &[(usize, Instruction)],
    leaders: &[usize],
) -> Vec<BasicBlock> {
    let mut blocks = Vec::with_capacity(leaders.len());
    let mut current_start_pc = instructions[0].0;
    let mut current_start_idx = 0;

    let mut leader_idx = 1; // leaders[0] is always instructions[0].0, handled by current_start_pc

    for (i, (pc, _)) in instructions.iter().enumerate() {
        if leader_idx < leaders.len() && *pc == leaders[leader_idx] {
            if i > current_start_idx {
                blocks.push(BasicBlock {
                    start_pc: current_start_pc,
                    end_pc: *pc,
                    instructions: instructions[current_start_idx..i].to_vec(),
                });
                current_start_idx = i;
                current_start_pc = *pc;
            }
            leader_idx += 1;
        } else if leader_idx < leaders.len() && *pc > leaders[leader_idx] {
            // In case a leader points to a PC that doesn't exist (e.g. invalid bytecode),
            // skip to the next valid leader or use binary search
        }
    }
    // ...
```
Wait, if a leader points to a PC that doesn't exist (e.g. invalid branch), `*pc == leaders[leader_idx]` might get stuck if we just increment `leader_idx`.
Using `binary_search` is safer and very fast (O(log L)):
```rust
        if leaders.binary_search(pc).is_ok() && i > current_start_idx {
```
Yes! `binary_search` on a contiguous `Vec` is extremely cache-friendly and faster than `BTreeSet::contains`.

Let's test this!
