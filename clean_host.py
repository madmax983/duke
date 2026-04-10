import re

with open("crates/duke-gc/src/host.rs", "r") as f:
    content = f.read()

# Make the fields pub(crate) instead of pub
content = content.replace("pub host_files:", "pub(crate) host_files:")
content = content.replace("pub next_host_file_id:", "pub(crate) next_host_file_id:")

# Clean up duplicate #[cfg(not(tarpaulin_include))] and whitespace
content = re.sub(r'(#\[cfg\(not\(tarpaulin_include\)\)\]\s*)+', r'#[cfg(not(tarpaulin_include))]\n', content)
content = re.sub(r'(#\[allow\(unexpected_cfgs\)\]\s*)+', r'#[allow(unexpected_cfgs)]\n', content)
content = re.sub(r'\n{4,}', '\n\n', content)

with open("crates/duke-gc/src/host.rs", "w") as f:
    f.write(content)
