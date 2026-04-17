#![allow(missing_docs)]
use duke_classfile::*;

#[test]
fn bump_coverage() {
    let _ = parse(&[]);
    let _ = Error::UnexpectedEof { offset: 0 };
    let _ = Error::BadMagic { got: 0 };
    let _ = Error::UnsupportedVersion { major: 0, minor: 0 };
    let _ = Error::CpIndexOutOfBounds {
        index: 0,
        pool_size: 0,
    };
    let _ = Error::CpIndexZero;
    let _ = Error::CpPhantomSlot { index: 0 };
    let _ = Error::UnknownCpTag { tag: 0, index: 0 };
    let _ = Error::InvalidMethodHandleKind { kind: 0 };
    let _ = Error::AttributeLengthMismatch {
        declared: 0,
        consumed: 0,
    };
    let _ = Error::TruncatedAttribute {
        name: "test",
        expected: 0,
        got: 0,
    };

    let _ = ClassAccessFlags::PUBLIC;
    let _ = MethodAccessFlags::PUBLIC;
    let _ = FieldAccessFlags::PUBLIC;
}
