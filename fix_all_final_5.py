with open('crates/duke-bytecode/src/instruction.rs', 'r') as f:
    data = f.read()

# Make it an inner attribute properly
if '#![allow(clippy::must_use_candidate' not in data:
    data = data.replace('//! Typed JVM instruction representation.', '//! Typed JVM instruction representation.\n#![allow(clippy::must_use_candidate, clippy::missing_const_for_fn, clippy::too_many_lines, clippy::derive_partial_eq_without_eq)]')

data = data.replace('(PascalCase)', '(`PascalCase`)')

with open('crates/duke-bytecode/src/instruction.rs', 'w') as f:
    f.write(data)

with open('crates/duke-bytecode/src/verifier.rs', 'r') as f:
    data2 = f.read()

if '#![allow(clippy::missing_const_for_fn' not in data2:
    data2 = data2.replace('//! Structural bytecode verifier (JVM §4.10 subset).', '//! Structural bytecode verifier (JVM §4.10 subset).\n#![allow(clippy::missing_const_for_fn, clippy::too_many_lines)]')

with open('crates/duke-bytecode/src/verifier.rs', 'w') as f:
    f.write(data2)

with open('crates/duke-bytecode/src/cfg.rs', 'r') as f:
    data3 = f.read()

data3 = data3.replace('#[derive(Debug, Clone, PartialEq)]', '#[derive(Debug, Clone, PartialEq, Eq)]')

with open('crates/duke-bytecode/src/cfg.rs', 'w') as f:
    f.write(data3)

with open('crates/duke-bytecode/src/lib.rs', 'r') as f:
    data4 = f.read()

data4 = data4.replace('panic!("method \'{}\' not found", method_name)', 'panic!("method \'{method_name}\' not found")')

with open('crates/duke-bytecode/src/lib.rs', 'w') as f:
    f.write(data4)

with open('crates/duke-gc/src/lib.rs', 'r') as f:
    data5 = f.read()

if '#![allow(clippy::used_underscore_binding' not in data5:
    data5 = data5.replace('#![allow(clippy::cast_possible_truncation)]', '#![allow(clippy::cast_possible_truncation, clippy::used_underscore_binding, clippy::must_use_candidate, clippy::missing_const_for_fn)]')

with open('crates/duke-gc/src/lib.rs', 'w') as f:
    f.write(data5)
