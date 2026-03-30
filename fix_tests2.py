import re

with open("crates/duke-classfile/src/parser.rs", "r") as f:
    content = f.read()

# Make sure we only have one test mod.
content = re.sub(r'#\[cfg\(test\)\]\s*mod cp_dynamic_module_tests \{', '#[cfg(test)]\nmod tests {', content)
content = re.sub(r'\}\s*\n#\[cfg\(test\)\]\s*mod cp_tests \{\s*\n    use super::\*\;', '', content)
content = re.sub(r'\}\s*\n    use super::\*\;', '', content)

with open("crates/duke-classfile/src/parser.rs", "w") as f:
    f.write(content)
