with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    content = f.read()

replacements = [
    ("/// Boolean array (T_BOOLEAN = 4)", "/// Boolean array (`T_BOOLEAN` = 4)"),
    ("/// Char array (T_CHAR = 5)", "/// Char array (`T_CHAR` = 5)"),
    ("/// Float array (T_FLOAT = 6)", "/// Float array (`T_FLOAT` = 6)"),
    ("/// Double array (T_DOUBLE = 7)", "/// Double array (`T_DOUBLE` = 7)"),
    ("/// Byte array (T_BYTE = 8)", "/// Byte array (`T_BYTE` = 8)"),
    ("/// Short array (T_SHORT = 9)", "/// Short array (`T_SHORT` = 9)"),
    ("/// Int array (T_INT = 10)", "/// Int array (`T_INT` = 10)"),
    ("/// Long array (T_LONG = 11)", "/// Long array (`T_LONG` = 11)"),
    ("    #[must_use]\n    #[must_use]", "    #[must_use]"),
    ("    #[must_use]\n    /// Converts", "    /// Converts"),
    ("impl ArrayType {\n    /// Converts a raw JVM array type code into an `ArrayType`.\n    #[must_use]\n    pub const fn from_u8(v: u8) -> Option<Self> {", "impl ArrayType {\n    /// Converts a raw JVM array type code into an `ArrayType`.\n    #[must_use]\n    pub const fn from_u8(v: u8) -> Option<Self> {")
]

for old, new in replacements:
    content = content.replace(old, new)

import re
content = re.sub(r'#\[must_use\]\n\s*#\[must_use\]', '#[must_use]', content)

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(content)
