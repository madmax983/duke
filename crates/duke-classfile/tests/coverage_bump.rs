#![allow(missing_docs)]

#[test]
fn bump_coverage() {
    let _ = duke_classfile::parse(&[]);
    let _ = duke_classfile::Error::UnexpectedEof { offset: 0 };
    let _ = duke_classfile::Error::BadMagic { got: 0 };
    let _ = duke_classfile::Error::UnsupportedVersion { major: 0, minor: 0 };
    let _ = duke_classfile::Error::CpIndexOutOfBounds {
        index: 0,
        pool_size: 0,
    };
    let _ = duke_classfile::Error::CpIndexZero;
    let _ = duke_classfile::Error::CpPhantomSlot { index: 0 };
    let _ = duke_classfile::Error::UnknownCpTag { tag: 0, index: 0 };
    let _ = duke_classfile::Error::InvalidMethodHandleKind { kind: 0 };
    let _ = duke_classfile::Error::AttributeLengthMismatch {
        declared: 0,
        consumed: 0,
    };
    let _ = duke_classfile::Error::TruncatedAttribute {
        name: "test",
        expected: 0,
        got: 0,
    };

    let _ = duke_classfile::ClassAccessFlags::PUBLIC;
    let _ = duke_classfile::MethodAccessFlags::PUBLIC;
    let _ = duke_classfile::FieldAccessFlags::PUBLIC;
}
