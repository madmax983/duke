import re

with open("crates/duke-interpreter/src/native.rs", "r") as f:
    content = f.read()

# Bard memory:
# "If a core API is undocumented, write the guide."
# "The 'Black Box': Complex modules with no Module-Level (//!) documentation explaining the high-level concept."
# "Use //! at the top of files to explain what this file does before listing how."

# I'll just add /// to the undocumented pub(crate) fn in native.rs using a script, maybe some of them. Wait, there are 30+ undocumented functions.
# I'll add documentation to the zip and string ones.

# Let's add /// to native_println_string and friends.
replacements = [
    ("pub(crate) fn native_println_string(", "/// Native handler for `System.out.println(String)`.\n///\n/// Prints the given string to the output stream.\n///\n/// # Examples\n///\n/// ```no_run\n/// // Called internally when `System.out.println(\"Hello\")` is executed.\n/// ```\npub(crate) fn native_println_string("),
    ("pub(crate) fn native_println_int(", "/// Native handler for `System.out.println(int)`.\n///\n/// Prints the integer to the output stream.\n///\n/// # Examples\n///\n/// ```no_run\n/// // Called internally when `System.out.println(42)` is executed.\n/// ```\npub(crate) fn native_println_int("),
    ("pub(crate) fn native_println_void(", "/// Native handler for `System.out.println()`.\n///\n/// Prints a newline to the output stream.\n///\n/// # Examples\n///\n/// ```no_run\n/// // Called internally when `System.out.println()` is executed.\n/// ```\npub(crate) fn native_println_void("),
]

for old, new in replacements:
    content = content.replace(old, new)

with open("crates/duke-interpreter/src/native.rs", "w") as f:
    f.write(content)
