with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.startswith(') -> VmResult<Option<Slot>> {'):
        # We need to delete lines starting from the one right after bootstrap_stdlib's closing brace.
        # Wait, the problem is `native_println_string` was partially removed because of the first regex failure?
        # Let's inspect line 2083-2087
        for j in range(i-5, i+5):
            print(f"{j}: {lines[j].strip()}")
        break
