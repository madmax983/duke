with open("crates/duke-gc/src/host.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "#[cfg(not(unix))]" in line or "#[cfg(unix)]" in line:
        lines[i] = "#[cfg(not(tarpaulin_include))]\n" + lines[i]

with open("crates/duke-gc/src/host.rs", "w") as f:
    f.writelines(lines)
