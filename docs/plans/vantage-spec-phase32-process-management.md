# 🔭 Vantage: Spec for Process Management (`java.lang.ProcessBuilder`, `java.lang.Runtime.exec`)

## 👤 User Story
"As a System Automator running on Duke, I want to spawn and interact with native OS processes, so that my Java application can execute shell scripts, command-line utilities, or orchestrate external binaries without leaving the JVM."

## ❓ The "So What?"
What business problem does this solve?
Many modern applications are not isolated monoliths—they need to act as orchestrators. CI/CD agents, build tools (like Maven or Gradle), and data-processing pipelines routinely shell out to external programs (e.g., `git`, `ffmpeg`, or `docker`). Without the ability to spawn external processes, Duke cannot run applications that rely on the underlying operating system's tools. Adding native support for `ProcessBuilder` transforms Duke from a closed sandbox into a capable system-level orchestrator.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully spawn an external process (e.g., `ls` or `echo`), read its standard output via `Process.getInputStream()`, wait for its completion using `Process.waitFor()`, and retrieve the correct integer exit code.

## 🔍 Gap Analysis
- **Current State:** Duke has no mechanism for creating native operating system processes. All execution is strictly confined to interpreting JVM bytecode within the Duke process itself.
- **Market/Standard Lib:** The standard JVM heavily utilizes OS-specific native code (POSIX `fork`/`exec` on Unix, `CreateProcess` on Windows) via `java.lang.ProcessImpl` to spawn child processes, pipe standard I/O streams, and reap exit statuses.
- **The Gap:** We need native bridges to interface with the host OS. Rust's `std::process::Command` provides an excellent, cross-platform foundation for this. We must expose this capability to Java space, mapping Java's string arrays for commands and environments to Rust's `Command` API, and wrapping the resulting standard I/O handles into Java `InputStream` and `OutputStream` objects.

## ✅ Acceptance Criteria
- Must implement native bridges for `java.lang.ProcessImpl.start` (or the equivalent underlying native method for the implemented Java version).
- Must correctly map the Java command string array (`String[]`) and working directory to the OS-level process invocation.
- Must support redirecting standard output, standard error, and standard input between the child process and the Duke VM.
- Must implement `Process.waitFor()` to block the calling Java thread until the child process terminates, returning the exit code.
- Must implement `Process.destroy()` to forcibly terminate the child process.
- Must safely handle and throw `java.io.IOException` if the target executable cannot be found or lacks execution permissions.

## 🚫 Out of Scope
- Complex process pipelines (`ProcessBuilder.startPipeline()`).
- Process trees or recursive termination of child processes of child processes (handling zombie sub-processes).
- Inheriting the full Duke VM standard I/O streams by default without explicit redirection requests from the Java code.
