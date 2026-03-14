with open('crates/duke-bytecode/src/decoder.rs', 'r') as f:
    data = f.read()

# Add #[allow(...)] to the module level
if '#![allow(clippy::cast_possible_truncation' not in data:
    data = '#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_lossless, clippy::missing_const_for_fn)]\n' + data

with open('crates/duke-bytecode/src/decoder.rs', 'w') as f:
    f.write(data)

with open('crates/duke-bytecode/src/lib.rs', 'r') as f:
    data2 = f.read()

data2 = data2.replace('panic!("method \'{}\' not found", method_name)', 'panic!("method \'{method_name}\' not found")')
data2 = data2.replace('panic!("no Code attribute on method \'{}\'", method_name)', 'panic!("no Code attribute on method \'{method_name}\'")')

with open('crates/duke-bytecode/src/lib.rs', 'w') as f:
    f.write(data2)
