pub mod error;
pub mod frame;
pub mod slot;

pub use error::{VmError, VmResult};
pub use frame::Frame;
pub use slot::Slot;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_int_round_trip() {
        let s = Slot::Int(42);
        assert_eq!(s.as_int().unwrap(), 42);
    }

    #[test]
    fn frame_push_pop() {
        let mut frame = Frame::new(4, 2, vec![]).expect("new frame");
        frame.push(Slot::Int(7)).expect("push");
        assert_eq!(frame.pop_int().expect("pop"), 7);
    }

    #[test]
    fn frame_locals_from_args() {
        let frame =
            Frame::new(4, 3, vec![Slot::Int(1), Slot::Int(2), Slot::Int(3)]).expect("new frame");
        assert_eq!(frame.load_local(0).unwrap(), Slot::Int(1));
        assert_eq!(frame.load_local(2).unwrap(), Slot::Int(3));
    }

    #[test]
    fn frame_stack_overflow() {
        let mut frame = Frame::new(1, 1, vec![]).expect("new frame");
        frame.push(Slot::Int(1)).expect("push 1");
        let err = frame.push(Slot::Int(2)).unwrap_err();
        assert!(matches!(err, VmError::StackOverflow));
    }

    #[test]
    fn frame_stack_underflow() {
        let mut frame = Frame::new(4, 1, vec![]).expect("new frame");
        let err = frame.pop().unwrap_err();
        assert!(matches!(err, VmError::StackUnderflow));
    }
}
