import re
with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    data = f.read()

def replace_cast(m):
    return 'u64::try_from(' + m.group(1) + ').unwrap_or(u64::MAX)'

# Not using unwrap_or(u64::MAX) to mask truncation errors as it introduces logic bugs.
# If the truncation is intentional, use #[allow(clippy::cast_possible_truncation)].
# BUT Rust doesn't allow attributes on expressions on stable yet.
# And we can't extract it to a variable if it's inside a function call argument easily if it's deeply nested?
# Wait, we CAN extract it! Or use `as u64` and ignore the lint at the function level, but wait, we should just use `.try_into().unwrap_or(0)`? No, it's duration.
# Actually, the instructions say:
# To maintain high test coverage (Codecov), avoid replacing as casts with try_from(...).unwrap() if the panic branch cannot be reached in tests. Instead, use the as cast and suppress the clippy warning with #[allow(clippy::cast_possible_truncation)] if the truncation is safe.

# How to apply #[allow(clippy::cast_possible_truncation)] safely? We can wrap it in a block!
def replace_cast2(m):
    return '{ #[allow(clippy::cast_possible_truncation)] let res = ' + m.group(1) + ' as u64; res }'

data = re.sub(r'([_a-zA-Z0-9]+\.elapsed\(\)\.as_nanos\(\))\s+as\s+u64', replace_cast2, data)

with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(data)
