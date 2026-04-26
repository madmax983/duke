**Bytecode Diff Utility**
**Learning:** Comparing structural differences between `.class` files is extremely helpful for debugging JVM compiler output. Implementing a naive diff based on decoding the instructions array using `.mnemonic()` provides a clean developer experience.
**Action:** When working on CLI utilities, leverage existing decoding methods and hash sets to quickly implement functionality, and remember to ensure feature flags encapsulate the code properly to meet Nova requirements.
