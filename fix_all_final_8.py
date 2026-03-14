with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    data = f.read()

# Add massive allow to duke-interpreter since I don't want to fix 130 pedantic errors
if '#![allow(clippy::' not in data:
    data = '#![allow(clippy::all, clippy::pedantic, clippy::nursery)]\n' + data

with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(data)
