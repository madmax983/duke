# 🔭 Vantage: Spec for Java Agents (java.lang.instrument)

## 👤 User Story
"As an APM Vendor or Platform Engineer, I want the ability to dynamically instrument bytecode at load-time using Java Agents, so that I can inject tracing, metrics, and logging into third-party libraries without modifying their source code."

## ❓ The "So What?"
What business problem does this solve?
Modern enterprise Java applications rely heavily on Application Performance Monitoring (APM) tools like Datadog, New Relic, and OpenTelemetry. These tools work by attaching Java Agents (`-javaagent`) that rewrite bytecode to inject telemetry automatically. Without `java.lang.instrument` support, Duke cannot run these essential APM agents. Supporting Java Agents allows Duke to seamlessly integrate into existing enterprise observability stacks, making it a viable runtime for mission-critical deployments.

## 📈 Metric Definition
Success = An APM agent (e.g., OpenTelemetry Java Agent) can be attached via `-javaagent` at startup, intercept class loading, and successfully modify the bytecode of a target class (like `HttpServlet`) before it is defined.

## 🔍 Gap Analysis
- **Current State:** Duke parses, loads, and executes class files directly. There is no hook for pre-processing bytecode between loading and linking/defining.
- **Market/Standard Lib:** Standard JVMs provide the `java.lang.instrument.Instrumentation` API and support `-javaagent` command-line arguments to register `ClassFileTransformer` instances.
- **The Gap:** Duke needs an interception point in the `duke-loader` pipeline and a native implementation of the `Instrumentation` interface to allow registered agents to inspect and modify bytecode.

## ✅ Acceptance Criteria
- Must support the `-javaagent:/path/to/agent.jar=options` command line flag.
- Must invoke the agent's `premain` method before the application's `main` method.
- Must provide a native implementation of `java.lang.instrument.Instrumentation`.
- Must allow agents to register `ClassFileTransformer` instances.
- Must invoke registered transformers to allow bytecode modification before a class is defined by the VM.

## 🚫 Out of Scope
- Dynamic attachment of agents to a running JVM (e.g., via Attach API) - Phase 2.
- Retransformation of already loaded classes (`Instrumentation.retransformClasses`) - Phase 2.
- Native agents (JVMTI) - we are only focusing on Java Agents for now.
