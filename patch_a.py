import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    text = f.read()

# I also noticed there are blocks like:
# let a = match args.get(0) {
#     Some(s) => ...val(s)?,
#     None => return Err(VmError::NullPointerException),
# };
for val_fn in ["str_val", "int_val", "long_val", "float_val", "bool_val", "byte_val", "short_val", "char_val", "double_val"]:
    old_a = f"""let a = match args.get(0) {{
        Some(s) => {val_fn}(s)?,
        None => return Err(VmError::NullPointerException),
    }};"""
    new_a = f"let a = {val_fn}(args.get(0).ok_or(VmError::NullPointerException)?)?;"
    text = text.replace(old_a, new_a)

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(text)
