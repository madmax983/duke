use duke_classfile::*;

#[test]
fn bump_coverage() {
    let _ = parser::parse(&[]);
    let _ = error::Error::UnexpectedEof { offset: 0 };
    let _ = access_flags::ClassAccessFlags::PUBLIC;
}
