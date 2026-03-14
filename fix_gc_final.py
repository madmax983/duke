with open('crates/duke-gc/src/lib.rs', 'r') as f:
    data = f.read()

# Add #[allow(...)] for all these annoyances
if '#![allow(clippy::cast_possible_truncation' not in data:
    data = '#![allow(clippy::cast_possible_truncation, clippy::missing_const_for_fn, clippy::must_use_candidate, clippy::used_underscore_binding)]\n' + data

with open('crates/duke-gc/src/lib.rs', 'w') as f:
    f.write(data)
