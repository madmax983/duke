import re

with open("crates/duke-interpreter/src/native.rs", "r") as f:
    content = f.read()

# Fix clippy::uninlined_format_args
content = content.replace(
    """let _ = write!(&mut joined, "{}", n);""",
    """let _ = write!(&mut joined, "{n}");"""
)

# Fix clippy::single_match
#     match args.get(1) {
#         Some(Slot::Reference(Some(arr_ref))) => {
# ...
#         }
#         _ => {}
#     };
# This should be an `if let Some(Slot::Reference(Some(arr_ref))) = args.get(1)`
# but the replace is a bit tricky.
