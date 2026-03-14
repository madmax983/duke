import re

with open('crates/duke-classfile/src/parser.rs', 'r') as f:
    data = f.read()

s1 = """    match pool.get(i) {
        None => Err(ParseError::CpIndexOutOfBounds {
            index: idx.0,
            pool_size: pool.len(),
        }),
        Some(None) => Err(ParseError::CpPhantomSlot { index: idx.0 }),
        Some(Some(CpEntry::Utf8(s))) => Ok(s.as_str()),
        Some(Some(_)) => Err(ParseError::CpIndexOutOfBounds {
            index: idx.0,
            pool_size: pool.len(),
        }),
    }"""

s2 = """    match pool.get(i) {
        Some(None) => Err(ParseError::CpPhantomSlot { index: idx.0 }),
        Some(Some(CpEntry::Utf8(s))) => Ok(s.as_str()),
        None | Some(Some(_)) => Err(ParseError::CpIndexOutOfBounds {
            index: idx.0,
            pool_size: pool.len(),
        }),
    }"""

data = data.replace(s1, s2)
with open('crates/duke-classfile/src/parser.rs', 'w') as f:
    f.write(data)
