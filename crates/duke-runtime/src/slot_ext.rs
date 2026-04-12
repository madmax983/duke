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
