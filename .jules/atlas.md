## 2024-05-24 - Wildcard Exports in duke-classfile
**Tangle:** The `duke-classfile` crate used wildcard exports (`pub use crate::attributes::*;`, etc.) in its public API facade, creating a 'Leaky Abstraction' that could unintentionally expose internal implementation details or cause namespace pollution.
**Blueprint:** Replaced the wildcard exports with explicit, itemized re-exports (`pub use crate::attributes::{...};`) to strictly enforce the public API boundary as a concrete contract.
