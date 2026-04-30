#[cfg(test)]
mod tests {
    use crate::frame::Frame;
    use crate::slot::Slot;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn does_not_crash_frame_creation(
            max_stack in 0usize..1000,
            max_locals in 0usize..1000,
            args_len in 0usize..1000,
        ) {
            let mut args = Vec::new();
            for _ in 0..args_len {
                args.push(Slot::Int(0));
            }
            let _ = Frame::new(max_stack, max_locals, args);
        }

        #[test]
        fn does_not_crash_frame_pop_underflow(
            max_stack in 0usize..10,
            max_locals in 0usize..10,
        ) {
            let mut frame = Frame::new(max_stack, max_locals, vec![]).unwrap();
            let _ = frame.pop(); // Expected to safely error if empty
            let _ = frame.pop_int();
            let _ = frame.pop_ref();
        }

        #[test]
        fn does_not_crash_frame_push_overflow(
            max_stack in 1usize..10,
            max_locals in 1usize..10,
        ) {
            let mut frame = Frame::new(max_stack, max_locals, vec![]).unwrap();
            for _ in 0..20 {
                let _ = frame.push(Slot::Int(1));
            }
        }

    }
}
