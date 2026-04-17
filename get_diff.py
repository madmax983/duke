import re

with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    content = f.read()

new_content = content.replace(
"""    pub const fn is_unconditional_jump(&self) -> bool {""",
"""    /// Returns `true` if the instruction is a switch statement.
    #[must_use]
    pub const fn is_switch(&self) -> bool {
        matches!(self, Self::Tableswitch { .. } | Self::Lookupswitch { .. })
    }

    /// Returns the target offset for the default case and a list of branch targets for a switch statement.
    #[must_use]
    pub fn switch_targets(&self) -> Option<(isize, Vec<(String, isize)>)> {
        match self {
            Self::Tableswitch {
                default,
                low,
                high: _,
                offsets,
            } => {
                let mut targets = Vec::with_capacity(offsets.len());
                for (i, offset) in offsets.iter().enumerate() {
                    targets.push(((i as i32 + *low).to_string(), *offset as isize));
                }
                Some((*default as isize, targets))
            }
            Self::Lookupswitch { default, pairs } => {
                let mut targets = Vec::with_capacity(pairs.len());
                for (key, offset) in pairs {
                    targets.push((key.to_string(), *offset as isize));
                }
                Some((*default as isize, targets))
            }
            _ => None,
        }
    }

    /// Returns `true` if the instruction is an unconditional jump.""")

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(new_content)
