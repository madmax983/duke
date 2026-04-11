import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    text = f.read()

# We know the tests module is entirely contained within #[cfg(test)]\nmod tests { ... }
# but fuzz is also test-gated: #[cfg(test)]\nmod fuzz;

# Let's just find the start of tests
match = re.search(r"(\n#\[cfg\(test\)\]\nmod tests \{)", text)
if match:
    start_idx = match.start(1)

    # We will just write out the test lines to tests.rs but KEEP it inline, we will use include!("tests.rs");
    # instead of doing mod tests; so it physically splits but rustc treats it as inline.
