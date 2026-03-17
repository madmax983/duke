## 2024-05-24 - The Missing ClassLoader Guides
**Confusion:** The `ClassLoader` implementations (`BootstrapLoader`, `DirectoryLoader`, and `JImageReader`) had no executable examples explaining how to instantiate them. Users were left guessing how `java/lang/Object` translates to a path under the hood.
**Clarification:** Added executable `/// # Examples` doc-tests for `new` and `open` methods on each struct demonstrating correct usage and error handling.
