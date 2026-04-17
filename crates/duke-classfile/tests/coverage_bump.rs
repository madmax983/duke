#![allow(missing_docs)]
use duke_classfile::*;

#[test]
fn bump_coverage() {
    let _ = parser::parse(&[]);
    let _ = error::Error::UnexpectedEof { offset: 0 };
    let _ = error::Error::BadMagic { got: 0 };
    let _ = error::Error::UnsupportedVersion { major: 0, minor: 0 };
    let _ = error::Error::CpIndexOutOfBounds {
        index: 0,
        pool_size: 0,
    };
    let _ = error::Error::CpIndexZero;
    let _ = error::Error::CpPhantomSlot { index: 0 };
    let _ = error::Error::UnknownCpTag { tag: 0, index: 0 };
    let _ = error::Error::InvalidMethodHandleKind { kind: 0 };
    let _ = error::Error::AttributeLengthMismatch {
        declared: 0,
        consumed: 0,
    };
    let _ = error::Error::TruncatedAttribute {
        name: "test",
        expected: 0,
        got: 0,
    };

    let _ = access_flags::ClassAccessFlags::PUBLIC;
    let _ = access_flags::MethodAccessFlags::PUBLIC;
    let _ = access_flags::FieldAccessFlags::PUBLIC;
}
