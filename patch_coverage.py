import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Instead of passing the whole string to with_capacity which creates an un-hit error or branch line when tarpaulin traces it, let's simplify.
code = code.replace(
    r"""fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::with_capacity(params.len());""",
    r"""fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::with_capacity(params.len());"""
)

# Actually, the coverage drop was on `parse_arg_descriptors` since we just added `.len()`. Let's test it by adding unit tests for `parse_arg_descriptors` to lib.rs at the end of the existing tests.
