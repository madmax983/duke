with open("crates/duke-telemetry/src/helpers.rs", "r") as f:
    content = f.read()

content = content.replace("mod tests {", "#[cfg(test)]\nmod tests {")

with open("crates/duke-telemetry/src/helpers.rs", "w") as f:
    f.write(content)
