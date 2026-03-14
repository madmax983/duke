with open('crates/duke-gc/src/lib.rs', 'r') as f:
    data = f.read()

data = data.replace('no OLD_BIT', 'no `OLD_BIT`')
data = data.replace('with OLD_BIT', 'with `OLD_BIT`')
data = data.replace('on OLD_BIT', 'on `OLD_BIT`')
data = data.replace('roots (OLD_BIT set)', 'roots (`OLD_BIT` set)')

data = data.replace('for slot in obj.fields.iter_mut()', 'for slot in &mut obj.fields')
data = data.replace('for slot in patched.iter_mut()', 'for slot in &mut patched')

with open('crates/duke-gc/src/lib.rs', 'w') as f:
    f.write(data)
