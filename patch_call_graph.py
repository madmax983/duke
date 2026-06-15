import re

with open("crates/duke-bytecode/src/call_graph.rs", "r") as f:
    content = f.read()

# Replace extract_method_ref signature
content = content.replace(
    "fn extract_method_ref(cf: &ClassFile, idx: CpIndex) -> Option<(String, String, String)> {",
    "fn extract_method_ref(cf: &ClassFile, idx: CpIndex) -> Option<(&str, &str, &str)> {"
)

# Replace resolve_class_name().to_string()
content = content.replace(
    "let class_name = resolve_class_name(cf, *class_idx).to_string();",
    "let class_name = resolve_class_name(cf, *class_idx);"
)

# Replace cp_str().to_string()
content = content.replace(
    "let method_name = cp_str(cf, *name_index)?.to_string();",
    "let method_name = cp_str(cf, *name_index)?;"
)
content = content.replace(
    "let method_desc = cp_str(cf, *descriptor_index)?.to_string();",
    "let method_desc = cp_str(cf, *descriptor_index)?;"
)

with open("crates/duke-bytecode/src/call_graph.rs", "w") as f:
    f.write(content)
