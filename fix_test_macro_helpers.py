with open("crates/duke-telemetry/src/helpers.rs", "r") as f:
    content = f.read()

content = content.replace("mod tests {", "#[cfg(all(test, feature = \"telemetry\"))]\nmod tests {")
content = content.replace("#[cfg(all(test, feature = \"telemetry\"))]\n#[cfg(all(test, feature = \"telemetry\"))]", "#[cfg(all(test, feature = \"telemetry\"))]")

with open("crates/duke-telemetry/src/helpers.rs", "w") as f:
    f.write(content)
