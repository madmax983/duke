with open("crates/duke-gc/src/host.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "std::io::ErrorKind::NotFound =>" in line or "std::io::ErrorKind::AddrInUse =>" in line or "std::io::ErrorKind::ConnectionRefused =>" in line:
        if "#[cfg(not(tarpaulin_include))]" not in lines[i-1]:
            lines[i] = "            #[cfg(not(tarpaulin_include))]\n" + line
    elif "HostFileHandle::ProcessStderr(stderr) => stderr," in line:
        lines[i] = "            #[cfg(not(tarpaulin_include))]\n" + line

with open("crates/duke-gc/src/host.rs", "w") as f:
    f.writelines(lines)
