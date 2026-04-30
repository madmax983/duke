import re

with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    content = f.read()

# I will replace `#[allow(missing_docs)]` with nothing, but wait, the instruction enum is massive.
# Bard memory:
# "When acting under strict constraints that forbid auto-generated, low-value documentation (e.g., 'Bard' persona), do not write scripts to apply boilerplate comments just to pass the `missing_docs` lint. If a massive enum or module cannot be meaningfully documented, strategically apply `#[allow(missing_docs)]` to the specific item (e.g., `pub enum`) instead of polluting the codebase with noise."

