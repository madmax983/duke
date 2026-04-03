import re

with open("duke/src/main.rs", "r") as f:
    content = f.read()

# The error was "duplicate macro attributes" on #[test]
# Let's find and remove double #[test]
content = re.sub(r'#\[test\]\n\s*#\[test\]', '#[test]', content)

with open("duke/src/main.rs", "w") as f:
    f.write(content)
