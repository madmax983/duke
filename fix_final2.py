import sys

def fix():
    with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
        lines = f.readlines()

    extracted = lines[47:25550]

    # Write native.rs. Just write it! BUT do not make everything pub(crate).
    # Wait, instead of extracting from 47 to 25550, what if we just extract to a generic `native.rs` and make it an INCLUDE file?
    # include!("native.rs");
    # Then we don't have to change ANY visibility. It will literally be compiled as if it's in lib.rs.

    with open('crates/duke-interpreter/src/native.rs', 'w') as f:
        f.writelines(extracted)

    remaining = lines[:47] + [
        "include!(\"native.rs\");\n"
    ] + lines[25550:]

    with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
        f.writelines(remaining)

fix()
