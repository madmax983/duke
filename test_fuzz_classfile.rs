use duke_classfile::parse;

fn main() {
    let code: Vec<u8> = vec![
        0xca, 0xfe, 0xba, 0xbe, // magic
        0x00, 0x00, 0x00, 0x41, // version
        0x00, 0x06, // cp count
        // cp_entry[1]
        15, // MethodHandle
        0,  // invalid kind! 0
        0x00, 0x00
    ];
    let res = parse(&code);
    println!("{:?}", res);
}
