#[test]
fn havoc_test_oom_attr() {
    let bad_data = vec![
        0xca, 0xfe, 0xba, 0xbe, // MAGIC
        0x00, 0x00, // minor
        0x00, 0x3d, // major
        0x00, 0x02, // pool size = 2
        0x01, 0x00, 0x04, b'C', b'o', b'd', b'e', // pool entry 1 "Code"
        0x00, 0x00, // flags
        0x00, 0x01, // this
        0x00, 0x01, // super
        0x00, 0x00, // interfaces
        0x00, 0x00, // fields
        0x00, 0x01, // methods count 1
        // method 1
        0x00, 0x00, // access flags
        0x00, 0x01, // name index
        0x00, 0x01, // desc index
        0x00, 0x01, // attr count 1
        // attr 1
        0x00, 0x01, // name index "Code"
        0x00, 0x00, 0x00, 0x0e, // attr len
        // code attr
        0x00, 0x00, // max stack
        0x00, 0x00, // max locals
        0x00, 0x00, 0x00, 0x00, // code len
        0xff, 0xff, // ex_count = 65535
              // missing exception table bytes
    ];
    let res = duke_classfile::parse(&bad_data);
    assert!(res.is_err());
}

#[test]
fn havoc_test_oom_bootstrap_methods() {
    let bad_data = vec![
        0xca, 0xfe, 0xba, 0xbe, // MAGIC
        0x00, 0x00, // minor
        0x00, 0x3d, // major
        0x00, 0x02, // pool size = 2
        0x01, 0x00, 0x10, b'B', b'o', b'o', b't', b's', b't', b'r', b'a', b'p', b'M', b'e', b't',
        b'h', b'o', b'd', b's', // pool entry 1 "BootstrapMethods"
        0x00, 0x00, // flags
        0x00, 0x01, // this
        0x00, 0x01, // super
        0x00, 0x00, // interfaces
        0x00, 0x00, // fields
        0x00, 0x00, // methods count 0
        // class attrs
        0x00, 0x01, // attr count 1
        // attr 1
        0x00, 0x01, // name index "BootstrapMethods"
        0x00, 0x00, 0x00, 0x02, // attr len
        0xff, 0xff, // num = 65535
    ];
    let res = duke_classfile::parse(&bad_data);
    assert!(res.is_err());
}

#[test]
fn havoc_test_oom_lvt() {
    let bad_data = vec![
        0xca, 0xfe, 0xba, 0xbe, // MAGIC
        0x00, 0x00, // minor
        0x00, 0x3d, // major
        0x00, 0x02, // pool size = 2
        0x01, 0x00, 0x12, b'L', b'o', b'c', b'a', b'l', b'V', b'a', b'r', b'i', b'a', b'b', b'l',
        b'e', b'T', b'a', b'b', b'l', b'e', // pool entry 1 "LocalVariableTable"
        0x00, 0x00, // flags
        0x00, 0x01, // this
        0x00, 0x01, // super
        0x00, 0x00, // interfaces
        0x00, 0x00, // fields
        0x00, 0x00, // methods count 0
        // class attrs
        0x00, 0x01, // attr count 1
        // attr 1
        0x00, 0x01, // name index "LocalVariableTable"
        0x00, 0x00, 0x00, 0x02, // attr len
        0xff, 0xff, // len = 65535
    ];
    let res = duke_classfile::parse(&bad_data);
    assert!(res.is_err());
}

#[test]
fn havoc_test_oom_attributes() {
    let bad_data = vec![
        0xca, 0xfe, 0xba, 0xbe, // MAGIC
        0x00, 0x00, // minor
        0x00, 0x3d, // major
        0x00, 0x02, // pool size = 2
        0x01, 0x00, 0x04, b'C', b'o', b'd', b'e', // pool entry 1 "Code"
        0x00, 0x00, // flags
        0x00, 0x01, // this
        0x00, 0x01, // super
        0x00, 0x00, // interfaces
        0x00, 0x00, // fields
        0x00, 0x00, // methods count 0
        0xff, 0xff, // class attrs count = 65535
    ];
    let res = duke_classfile::parse(&bad_data);
    assert!(res.is_err());
}

#[test]
fn havoc_test_oom_bootstrap_methods_inner() {
    let bad_data = vec![
        0xca, 0xfe, 0xba, 0xbe, // MAGIC
        0x00, 0x00, // minor
        0x00, 0x3d, // major
        0x00, 0x02, // pool size = 2
        0x01, 0x00, 0x10, b'B', b'o', b'o', b't', b's', b't', b'r', b'a', b'p', b'M', b'e', b't',
        b'h', b'o', b'd', b's', // pool entry 1 "BootstrapMethods"
        0x00, 0x00, // flags
        0x00, 0x01, // this
        0x00, 0x01, // super
        0x00, 0x00, // interfaces
        0x00, 0x00, // fields
        0x00, 0x00, // methods count 0
        // class attrs
        0x00, 0x01, // attr count 1
        // attr 1
        0x00, 0x01, // name index "BootstrapMethods"
        0x00, 0x00, 0x00, 0x04, // attr len
        0x00, 0x01, // num = 1
        0x00, 0x00, // method ref
        0xff, 0xff, // num args = 65535
    ];
    let res = duke_classfile::parse(&bad_data);
    assert!(res.is_err());
}
