use crate::Slot;

pub trait SlotExt {
    fn unwrap_or_ref(self) -> Slot;
    fn unwrap_or_int(self) -> Slot;
    fn unwrap_or_long(self) -> Slot;
    fn unwrap_or_double(self) -> Slot;
}

impl SlotExt for Option<Slot> {
    #[inline]
    fn unwrap_or_ref(self) -> Slot {
        self.unwrap_or(Slot::Reference(None))
    }

    #[inline]
    fn unwrap_or_int(self) -> Slot {
        self.unwrap_or(Slot::Int(0))
    }

    #[inline]
    fn unwrap_or_long(self) -> Slot {
        self.unwrap_or(Slot::Long(0))
    }

    #[inline]
    fn unwrap_or_double(self) -> Slot {
        self.unwrap_or(Slot::Double(0.0))
    }
}

impl SlotExt for Option<&Slot> {
    #[inline]
    fn unwrap_or_ref(self) -> Slot {
        self.copied().unwrap_or(Slot::Reference(None))
    }

    #[inline]
    fn unwrap_or_int(self) -> Slot {
        self.copied().unwrap_or(Slot::Int(0))
    }

    #[inline]
    fn unwrap_or_long(self) -> Slot {
        self.copied().unwrap_or(Slot::Long(0))
    }

    #[inline]
    fn unwrap_or_double(self) -> Slot {
        self.copied().unwrap_or(Slot::Double(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_or_ref() {
        assert_eq!(
            Some(Slot::Reference(Some(1))).unwrap_or_ref(),
            Slot::Reference(Some(1))
        );
        assert_eq!(None::<Slot>.unwrap_or_ref(), Slot::Reference(None));

        let s = Slot::Reference(Some(1));
        assert_eq!(Some(&s).unwrap_or_ref(), Slot::Reference(Some(1)));
        assert_eq!(None::<&Slot>.unwrap_or_ref(), Slot::Reference(None));
    }

    #[test]
    fn test_unwrap_or_int() {
        assert_eq!(Some(Slot::Int(1)).unwrap_or_int(), Slot::Int(1));
        assert_eq!(None::<Slot>.unwrap_or_int(), Slot::Int(0));

        let s = Slot::Int(1);
        assert_eq!(Some(&s).unwrap_or_int(), Slot::Int(1));
        assert_eq!(None::<&Slot>.unwrap_or_int(), Slot::Int(0));
    }

    #[test]
    fn test_unwrap_or_long() {
        assert_eq!(Some(Slot::Long(1)).unwrap_or_long(), Slot::Long(1));
        assert_eq!(None::<Slot>.unwrap_or_long(), Slot::Long(0));

        let s = Slot::Long(1);
        assert_eq!(Some(&s).unwrap_or_long(), Slot::Long(1));
        assert_eq!(None::<&Slot>.unwrap_or_long(), Slot::Long(0));
    }

    #[test]
    fn test_unwrap_or_double() {
        assert_eq!(Some(Slot::Double(1.0)).unwrap_or_double(), Slot::Double(1.0));
        assert_eq!(None::<Slot>.unwrap_or_double(), Slot::Double(0.0));

        let s = Slot::Double(1.0);
        assert_eq!(Some(&s).unwrap_or_double(), Slot::Double(1.0));
        assert_eq!(None::<&Slot>.unwrap_or_double(), Slot::Double(0.0));
    }
}
