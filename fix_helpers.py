import re

with open("crates/duke-telemetry/src/helpers.rs", "r") as f:
    content = f.read()

replacements = [
    (
        "    pub fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>",
        "    /// Core helper: serialize any `HashMap<K, V>` by formatting each key with `key_fn`.\n    ///\n    /// ⚡ Bolt: Using `SerializeMap` to serialize directly removes the intermediate `HashMap`\n    /// collection, avoiding heap allocations and hashing overhead during telemetry generation.\n    pub fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>"
    ),
    (
        "    pub fn site3<V: Serialize, S: serde::Serializer>(",
        "    /// `HashMap<(class, method, pc), V>` → `\"class::method@pc\"`.\n    pub fn site3<V: Serialize, S: serde::Serializer>("
    ),
    (
        "    pub fn site2_u16<V: Serialize, S: serde::Serializer>(",
        "    /// `HashMap<(class, cp_idx), V>` → `\"class@cp\"`.\n    pub fn site2_u16<V: Serialize, S: serde::Serializer>("
    ),
    (
        "    pub fn pair_str<V: Serialize, S: serde::Serializer>(",
        "    /// `HashMap<(class, method), V>` → `\"class::method\"`.\n    pub fn pair_str<V: Serialize, S: serde::Serializer>("
    ),
    (
        "    pub fn sorted_set<S: serde::Serializer>(",
        "    /// Serialize `HashSet<String>` as a sorted `Vec<String>` for deterministic output.\n    pub fn sorted_set<S: serde::Serializer>("
    )
]

for old, new in replacements:
    content = content.replace(old, new)

with open("crates/duke-telemetry/src/helpers.rs", "w") as f:
    f.write(content)
