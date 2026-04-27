import re

with open("crates/duke-interpreter/src/native.rs", "r") as f:
    content = f.read()

# Fix clippy::uninlined_format_args
content = content.replace(
    'write!(&mut joined, "{}", n)',
    'write!(&mut joined, "{n}")'
)

# Fix clippy::single_match
old_match = """    match args.get(1) {
        Some(Slot::Reference(Some(arr_ref))) => {
            let obj = heap.get(*arr_ref)?;"""

new_match = """    if let Some(Slot::Reference(Some(arr_ref))) = args.get(1) {
        let obj = heap.get(*arr_ref)?;"""

content = content.replace(old_match, new_match)

old_match_end = """                }
            }
        }
        _ => {}
    };

    let r = heap.allocate_string(joined);"""

new_match_end = """                }
            }
        }

    let r = heap.allocate_string(joined);"""

content = content.replace(old_match_end, new_match_end)

with open("crates/duke-interpreter/src/native.rs", "w") as f:
    f.write(content)
