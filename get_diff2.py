import re

with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    data = f.read()

# Add why the optimization matters.
replacement = """    /// Returns the switch targets if the instruction is a switch statement.
    /// It returns `Some((default_offset, targets_iterator))`.
    ///
    /// ⚡ Bolt: By returning an iterator instead of allocating and collecting into
    /// a new `Vec`, we eliminate heap allocations during CFG generation and
    /// complexity calculation.
    #[must_use]"""

data = data.replace("""    /// Returns the switch targets if the instruction is a switch statement.
    /// It returns `Some((default_offset, targets_iterator))`.
    #[must_use]""", replacement)

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(data)
