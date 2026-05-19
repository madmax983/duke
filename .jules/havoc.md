**Fuzzing the Parser Layers**
**Learning:** Parsing raw byte structures (`manifest`, `jimage`, `classfile`) requires robust handling of potential UTF-8 failures. `std::str::from_utf8(bytes).unwrap()` or returning a hard panic on arbitrary bytes will eventually be hit by a fuzzer.
**Action:** Always prefer `String::from_utf8_lossy` or properly map the `Utf8Error` into the module's `Error` enumeration when parsing inputs that might contain malformed or malicious text blocks. I have set up fuzzing targets to continually test these layers.
