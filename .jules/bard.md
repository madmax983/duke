## 2024-05-24 - The Missing ClassLoader Guides
**Confusion:** The `ClassLoader` implementations (`BootstrapLoader`, `DirectoryLoader`, and `JImageReader`) had no executable examples explaining how to instantiate them. Users were left guessing how `java/lang/Object` translates to a path under the hood.
**Clarification:** Added executable `/// # Examples` doc-tests for `new` and `open` methods on each struct demonstrating correct usage and error handling.

## 2024-05-25 - Unescaped Brackets in Docstrings
**Confusion:** Developers often use brackets `[]` to represent array or map indices in docstrings (e.g., `fields[0]`). However, if these are not enclosed in backticks, `rustdoc` interprets them as intra-doc links, leading to noisy `unresolved link` warnings during `cargo doc`.
**Clarification:** Enclose all instances of bracketed indices or slice syntax (like `fields[0]` or `fields[1..size]`) in backticks (e.g., `` `fields[0]` ``) within doc comments `///` to prevent them from being parsed as markdown links.
