#[cfg(test)]
mod tests {
    use crate::decoder::decode;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn no_panic_on_arbitrary_bytecode(data in any::<Vec<u8>>()) {
            let _ = decode(&data);
        }
    }
}
