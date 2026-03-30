import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Make sure we didn't leave any `args.get(1).copied().unwrap_or(Slot::Reference(None))`
# Oh wait, we already saw they are gone except for `extract_slot_arg` definition.

# So the original `args.get(X).copied().unwrap_or(Slot::Reference(None))` were completely successfully replaced by `extract_slot_arg`! But the build failed because `extract_slot_arg` definition wasn't seen globally. It should be there now.
