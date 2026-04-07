with open("crates/duke-gc/src/host.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    # Add allow(unexpected_cfgs) along with cfg(not(tarpaulin_include))
    if "#[cfg(not(tarpaulin_include))]" in line:
        if i > 0 and "#[allow(unexpected_cfgs)]" in lines[i-1]:
            # we need to put it on the item, but match arms aren't items
            # let's just allow unexpected_cfgs at the module level
            pass

# To allow it at module level, add it to the top
lines.insert(0, "#![allow(unexpected_cfgs)]\n")

with open("crates/duke-gc/src/host.rs", "w") as f:
    f.writelines(lines)
