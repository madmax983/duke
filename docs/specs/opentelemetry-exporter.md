# 🔭 Vantage: Spec for OpenTelemetry Exporter

## 👤 User Story
As an Enterprise SRE or Platform Engineer, I want to stream Duke JVM runtime telemetry (bytecode execution cost, class init times, dispatch resolution) directly into OpenTelemetry (OTLP) compatible observability backends (like Prometheus, Jaeger, Datadog, or Grafana), so that I can monitor JVM health, trace execution paths, and set up alerts without building custom log parsers.

## 💡 What business problem does this solve?
Currently, Duke outputs its telemetry either as text/markdown reports or a raw JSON blob to standard output or a local file (`duke-telemetry` crate). In production enterprise environments, observability is centralized. Manually collecting, parsing, and ingesting custom JSON logs is fragile, lacks standardization, and creates unnecessary operational overhead. By adopting the industry-standard OTLP protocol, Duke becomes instantly compatible with the entire cloud-native observability ecosystem. This unlocks enterprise adoption by removing the integration barrier and providing immediate, out-of-the-box monitoring value.

## 📈 Success Metrics
- **Adoption:** 0 to 1 integration with an OTLP-compatible backend (e.g., successful ingestion into a local Jaeger or Prometheus instance).
- **Performance (Overhead):** The exporter must add < 2% performance overhead to the JVM execution time when active, and 0% overhead when disabled.
- **Reliability:** 0 panics during export under high load or network failure.

## 🔍 Gap Analysis
- **Standard Libs/Current State:** We currently have an in-memory `TelemetryStore` that dumps data periodically or at JVM shutdown via `to_json()` and `print_report()`. This is isolated to the local process.
- **The Market:** Modern runtimes (Node.js, Go, standard Java with agents) all provide first-class or standardized ways to emit OTLP data. Standard JVMs require complex java agents (like `opentelemetry-javaagent.jar`). Duke has a unique opportunity to build this directly into the native runtime layer.

## ✅ Acceptance Criteria
- Must implement an `OpentelemetryExporter` (or similar component) that can consume data from `TelemetryStore`.
- Must support exporting data via the standard OTLP HTTP or gRPC protocol.
- Must map Duke-specific metrics (e.g., `bytecode_cost`, `native_boundary` errors) to standard OpenTelemetry metric types (Counters, Histograms).
- Must map Duke exception flows and class initialization DAGs to OpenTelemetry Spans/Traces.
- Must handle network timeouts gracefully without panicking or blocking the main JVM execution threads.
- Must be configurable via standard `OTEL_*` environment variables (e.g., `OTEL_EXPORTER_OTLP_ENDPOINT`).

## 🚫 Out of Scope
- Writing custom dashboards for Grafana/Datadog (we just provide the data).
- Real-time bytecode manipulation for dynamic trace insertion (Phase 2).
- Replacing the existing JSON/Markdown reporters (they remain for local debugging).
