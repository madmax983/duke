# 🔭 Vantage: Spec for Lambda Captures

## 👤 User Story
"As a Java Developer running my code on Duke, I want the VM to support capturing variables in lambda expressions, so that I can use closures and pass local state into functional interfaces without encountering runtime panics."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's interpreter supports basic, stateless lambda expressions. However, it crashes with a `VmError::Unimplemented { mnemonic: "lambda capture missing" }` when a lambda attempts to capture a local variable from its enclosing scope. Capturing variables (closures) is a fundamental feature of modern Java (since Java 8) and is heavily used in standard library streams, reactive frameworks, and everyday application logic. Without this feature, developers are forced to refactor their code to use cumbersome anonymous inner classes or avoid streams entirely, which severely limits Duke's utility for modern Java workloads. Supporting lambda captures is essential for achieving functional parity with standard JVMs and unlocking the ability to run real-world Java code.

## 📈 Metric Definition
Success = A user can execute a Java program containing a lambda expression that captures at least one local variable from its enclosing method, and the program completes successfully without throwing an `Unimplemented` exception or producing incorrect results.

## 🔍 Gap Analysis
- **Current State:** Duke's native implementation of lambda instantiation (`java/lang/invoke/LambdaMetafactory.metafactory`) does not correctly populate the captured arguments into the generated lambda object's fields. The `crates/duke-interpreter/src/native.rs` file explicitly returns an `Unimplemented` error when iterating over `capture_index`.
- **Market/Standard Lib:** Standard JVMs dynamically generate a class for each lambda, where captured variables are passed to the constructor and stored as fields. When the lambda's functional method is invoked, these fields are prepended to the invocation arguments.
- **The Gap:** We need to update the lambda bridging logic in `native.rs` to correctly read the captured variables from the `InvokeDynamic` instruction's arguments (the stack) and store them in the dynamically allocated lambda object on the heap. Furthermore, the invocation logic must be updated to extract these captured fields and pass them to the underlying implementation method alongside the explicit arguments.

## ✅ Acceptance Criteria
- Must successfully execute lambdas that capture single and multiple local variables (primitives and object references).
- Must properly store captured variables in the dynamically allocated lambda object.
- Must correctly pass the captured variables to the target implementation method during invocation.
- Must eliminate the `VmError::Unimplemented { mnemonic: "lambda capture missing" }` error.

## 🚫 Out of Scope
- Support for `MethodHandle` combinators or complex `invokedynamic` use cases beyond standard lambda captures.
- Performance optimization of lambda object allocation (focus is on functional correctness first).
