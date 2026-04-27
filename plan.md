1. **Use `run_in_bash_session` to execute python scripts to update documentation**
   - Execute a python script to prepend `//!` documentation to `crates/duke-interpreter/src/native.rs`.
   - The script will add:
     ```rust
     //! JVM Native method implementations and JNI bridging.
     //!
     //! This module contains implementations for the `native` methods of standard
     //! library classes (e.g., `java/util/zip/ZipFile`, `java/lang/System`) and handles
     //! the boundary transitions between interpreted JVM bytecode and native Rust code.
     //!
     //! When `invoke_virtual` or `invoke_static` encounters a method marked `ACC_NATIVE`,
     //! control flow routes to the handlers registered here rather than decoding bytecode.
     ```
   - Execute a python script to add `///` doc comment to `pub fn execute_class` in `crates/duke-interpreter/src/native.rs`.
   - The script will add:
     ```rust
     /// Starts execution of a specific Java method within a given class.
     ///
     /// **Why it exists:** This function resolves the requested class and method, builds the
     /// initial execution frame, and begins interpreting instructions. It bridges the gap
     /// between the user requesting a class to run and the `execute` loop.
     ///
     /// # Arguments
     /// * `registry` - The class registry for resolving types.
     /// * `loader` - The class loader for fetching dependencies.
     /// * `heap` - The garbage collector heap.
     /// * `stdout` - The standard output stream.
     /// * `class_name` - The internal name of the class (e.g., `"java/lang/String"`).
     /// * `method_name` - The name of the method to execute (e.g., `"main"`).
     /// * `descriptor` - The method descriptor (e.g., `"([Ljava/lang/String;)V"`).
     /// * `args` - The arguments to pass to the method.
     ///
     /// # Returns
     ///
     /// * `Ok(Some(Slot))` - If the method completes and returns a value.
     /// * `Ok(None)` - If the method is `void` and completes.
     /// * `Err(Error)` - If the method throws an unhandled exception or encounters a fatal VM error.
     ///
     /// # Examples
     ///
     /// ```no_run
     /// # use duke_interpreter::{execute_class, ClassRegistry};
     /// # use duke_loader::bootstrap::BootstrapLoader;
     /// # use duke_gc::Heap;
     /// # let mut registry = ClassRegistry::new();
     /// # let loader = BootstrapLoader::new(std::path::Path::new("."), vec![std::path::Path::new(".")]).unwrap();
     /// # let mut heap = Heap::new();
     /// # let mut stdout = Vec::new();
     /// // Execute `public static void main(String[] args)`
     /// let result = execute_class(
     ///     &mut registry,
     ///     &loader,
     ///     &mut heap,
     ///     &mut stdout,
     ///     "com/example/App",
     ///     "main",
     ///     "([Ljava/lang/String;)V",
     ///     &[]
     /// );
     /// ```
     ```
   - Execute a python script to add `///` doc comment to `pub fn execute` in `crates/duke-interpreter/src/native.rs`.
   - The script will add:
     ```rust
     /// The core bytecode interpretation loop.
     ///
     /// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
     /// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
     ///
     /// # Arguments
     ///
     /// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
     /// * `cp` - The constant pool for the current class.
     /// * `args` - The initial local variables (arguments to the method).
     /// * `max_stack` - The maximum depth of the operand stack.
     /// * `max_locals` - The size of the local variable array.
     ///
     /// # Returns
     ///
     /// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
     /// * `Ok(None)` - If the method completes via `return` (void).
     /// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
     ///
     /// # Panics
     ///
     /// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
     /// state becomes invalid (e.g., stack underflow).
     ///
     /// # Examples
     ///
     /// ```
     /// use duke_bytecode::Instruction;
     /// use duke_runtime::Slot;
     /// use duke_interpreter::execute;
     ///
     /// let code = vec![
     ///     (0, Instruction::Iconst5),
     ///     (1, Instruction::Ireturn),
     /// ];
     /// let cp = vec![];
     /// let result = execute(&code, &cp, vec![], 1, 0).unwrap();
     /// assert_eq!(result, Some(Slot::Int(5)));
     /// ```
     ///
     /// # Errors
     ///
     /// Returns a `Error` if bytecode invariants are broken or if an exception is raised
     /// internally.
     ```

2. **Verify modifications with `read_file`**
   - Use `read_file` to confirm that the changes have been correctly applied without truncation or formatting issues.

3. **Verify modifications with `cargo doc`**
   - Use `run_in_bash_session` to run `cargo doc -p duke-interpreter --no-deps` to ensure that the documentation builds correctly.

4. **Run all relevant tests**
   - Use `run_in_bash_session` to run `cargo test -p duke-interpreter` to explicitly verify and guarantee that no regressions have been introduced.

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run the necessary pre-commit steps via `pre_commit_instructions`.

6. **Stage and Commit Changes**
   - Use `run_in_bash_session` to `git commit` the staged changes.

7. **Submit code review**
   - Request review with `request_code_review`.
