#![no_main]
use libfuzzer_sys::fuzz_target;
use duke_classfile::parse;
use duke_bytecode::decode;

fuzz_target!(|data: &[u8]| {
    // Just combine parse + decode to find potential bugs
    if let Ok(cf) = parse(data) {
        for method in cf.methods {
            for attr in method.attributes {
                if let duke_classfile::AttributeData::Code(code) = attr.data {
                    let _ = decode(&code.code);
                }
            }
        }
    }
});
