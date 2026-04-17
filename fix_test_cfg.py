import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

# Change #[cfg(test)]\n#[cfg(feature = "telemetry")] to just #[cfg(test)]
# Wait, actually let's see if the test functions themselves are running
