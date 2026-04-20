import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace(
    'store.object_lineage.record("java/lang/String", "Foo", "bar", 10);',
    'store.object_lineage.record("java/lang/String", "Foo", 10, "bar");'
)

content = content.replace(
    'store.dispatch_resolution.record_call("Foo", 42, "java/lang/String", "equals", 3);',
    'store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);\n        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);\n        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);'
)

content = content.replace(
    'store.native_boundary.record("java/lang/String", "intern", 100, true);',
    'store.native_boundary.record_call("java/lang/String", "intern", 100, true);'
)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
