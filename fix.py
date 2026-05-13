# The unresolved import in `duke/src/jar_diff.rs` is because it imports `duke_classfile::types::{...}`
# The AGENTS.md memory says: "Core types such as AttributeData, CpEntry, and CpIndex are exported directly at the crate root. Do not import them through a `types` submodule"
# Let's fix `duke/src/jar_diff.rs` to not use `types::`.
import re

with open("duke/src/jar_diff.rs", "r") as f:
    text = f.read()

text = text.replace("duke_classfile::{", "duke_classfile::{AttributeData, CpEntry, CpIndex, ")
text = text.replace("types::{AttributeData, CpEntry, CpIndex},", "")

# Let's fix the type inference errors as per the memory!
# "When chaining `.and_then(|x| x.as_ref())` on Options and the compiler throws `[E0282]: type annotations needed`, fix it by explicitly typing the closure argument (e.g., `.and_then(|slot: &Option<T>| slot.as_ref())`)."

# Let's replace `.and_then(|slot| slot.as_ref())` with `.and_then(|slot: &Option<CpEntry>| slot.as_ref())`
text = text.replace(".and_then(|slot| slot.as_ref())", ".and_then(|slot: &Option<CpEntry>| slot.as_ref())")
text = text.replace(".and_then(|s| s.as_ref())", ".and_then(|s: &Option<CpEntry>| s.as_ref())")

with open("duke/src/jar_diff.rs", "w") as f:
    f.write(text)
