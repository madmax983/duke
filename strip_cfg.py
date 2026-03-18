import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Remove `#[cfg(feature = "telemetry")]` lines everywhere in `lib.rs` EXCEPT inside `mod tests`
# Actually, wait, some blocks might be multi-line or just one statement.
# `#[cfg(feature = "telemetry")]` is often right before a single line.
# `#[cfg(feature = "telemetry")]\n                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(...)`
# We can just delete `#[cfg(feature = "telemetry")]` and any whitespace right before it, but be careful with indentation.

lines = code.split("\n")
new_lines = []
in_tests = False
for line in lines:
    if "mod tests {" in line:
        in_tests = True

    if "#[cfg(feature = \"telemetry\")]" in line:
        # If it's in the tests, maybe we keep it?
        # Actually in tests it's better to keep it if the test asserts on telemetry state which is only present when feature is enabled.
        # But wait! If the test asserts on it, the test will fail if the feature is disabled!
        # Let's see how tests use it. If a test is specifically testing telemetry, its whole `#[test]` is wrapped in `#[cfg(feature = "telemetry")]`.
        # So we should keep it for tests.
        if not in_tests:
            continue

    new_lines.append(line)

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write("\n".join(new_lines))
