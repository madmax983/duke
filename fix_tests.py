import re

with open("crates/duke-classfile/src/parser.rs", "r") as f:
    content = f.read()

# Replace both extra test modules with just mod tests

content = re.sub(r'#\[cfg\(test\)\]\s*mod cp_tests \{', '#[cfg(test)]\nmod tests {', content)
content = re.sub(r'#\[cfg\(test\)\]\s*mod cp_dynamic_module_tests \{\n    use super::\*\;\n\n', '', content)
content = re.sub(r'\}\n\n#\[cfg\(test\)\]\nmod tests \{', '', content)

with open("crates/duke-classfile/src/parser.rs", "w") as f:
    f.write(content)
