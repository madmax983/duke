#[cfg(test)]
mod tests {
    use duke_runtime::{Frame, Slot};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn does_not_crash_frame_methods(
            push_count in 0usize..50,
            pop_count in 0usize..50,
            push_value in any::<i32>(),
        ) {
            let mut frame = Frame::new(100, 100, vec![]).unwrap();
            for _ in 0..push_count {
                let _ = frame.push(Slot::Int(push_value));
            }
            for _ in 0..pop_count {
                let _ = frame.pop();
            }
        }
    }
}
