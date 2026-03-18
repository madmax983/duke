import re

with open("duke/Cargo.toml", "r") as f:
    content = f.read()

content = content.replace('telemetry = ["duke-interpreter/telemetry"]', 'telemetry = ["duke-interpreter/telemetry"]')

with open("duke/Cargo.toml", "w") as f:
    f.write(content)


with open("duke/src/main.rs", "r") as f:
    code = f.read()

# I want to find the emit_telemetry function and its cfg.

# Wait, `emit_telemetry` is already conditionally compiled:
# #[cfg(feature = "telemetry")]
# fn emit_telemetry(registry: &ClassRegistry, dest: Option<TelemetryDest>) { ... }
#
# #[cfg(not(feature = "telemetry"))]
# fn emit_telemetry(_registry: &ClassRegistry, dest: Option<TelemetryDest>) { ... }

# Since duke-interpreter's TelemetryStore.to_json() is only available when the feature is on, this is correct! We don't need to change `duke/src/main.rs` at all. The facade is correctly implemented now.
