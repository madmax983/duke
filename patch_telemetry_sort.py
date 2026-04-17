with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

replacements = [
    ("ops.sort_by(|a, b| b.1.count.cmp(&a.1.count));", "ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));"),
    ("sites.sort_by(|a, b| b.1.count.cmp(&a.1.count));", "sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));"),
    ("dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));", "dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));"),
    ("natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));", "natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));")
]

for s, r in replacements:
    content = content.replace(s, r)

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
