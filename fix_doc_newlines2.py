import re

with open('crates/duke-bytecode/src/error.rs', 'r') as f:
    content = f.read()

# Just delete the dangling doc comments since we removed the type aliases!
content = content.replace("/// A generic result type for operations returning a [`DecodeError`].\n\n", "")
content = content.replace("/// A generic result type for operations returning a [`VerifyError`].\n\n", "")
# if there is no \n\n, let's just delete the line
content = re.sub(r'/// A generic result type for operations returning a \[`DecodeError`\]\.\s+', '', content)
content = re.sub(r'/// A generic result type for operations returning a \[`VerifyError`\]\.\s+', '', content)

with open('crates/duke-bytecode/src/error.rs', 'w') as f:
    f.write(content)
