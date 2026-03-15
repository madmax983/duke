use crate::make_key;
use duke_runtime::{Slot, VmResult};
use std::collections::HashMap;
use std::io::Write;

/// Signature for native method implementations.
///
/// Arguments:
/// - `&[Slot]`: method arguments (including `this` in slot 0 for instance methods)
/// - `&mut Heap`: the object heap for reading/writing objects
/// - `&mut dyn Write`: output sink (stdout in production, Vec<u8> in tests)
pub type NativeHandler = fn(&[Slot], &mut duke_gc::Heap, &mut dyn Write) -> VmResult<Option<Slot>>;

/// A native handler that can call back into the interpreter to invoke Java methods.
///
/// The `invoke` closure takes `heap` and `output` as *parameters* (not captured),
/// using the "loan" pattern: the handler passes its borrows through each call and
/// gets them back when the call returns. Sequential reborrows — no unsafe required.
pub type CallbackNativeHandler = fn(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    invoke: &mut dyn FnMut(
        &mut duke_gc::Heap,
        &mut dyn Write,
        &str, // class name
        &str, // method name
        &str, // descriptor
        Vec<Slot>,
    ) -> VmResult<Option<Slot>>,
) -> VmResult<Option<Slot>>;

/// The `invoke` closure type passed into [`CallbackNativeHandler`] implementations.
///
/// Defined separately so function signatures that accept this parameter avoid the
/// `clippy::type_complexity` lint.
///
/// `pub` so that external crates can write their own [`CallbackNativeHandler`]
/// implementations.  If external registration is not needed, consider
/// narrowing to `pub(crate)`.
pub type InvokeFn<'a> = dyn FnMut(&mut duke_gc::Heap, &mut dyn Write, &str, &str, &str, Vec<Slot>) -> VmResult<Option<Slot>>
    + 'a;

/// Stored in `NativeRegistry` — all existing handlers stay `Simple`.
#[derive(Copy, Clone, Debug)]
pub enum HandlerKind {
    Simple(NativeHandler),
    Callback(CallbackNativeHandler),
}

/// Registry of native method implementations.
///
/// Maps `"class_name\x00method_name\x00descriptor"` to a handler kind.
pub struct NativeRegistry {
    handlers: HashMap<String, HandlerKind>,
}

impl NativeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    fn insert_handler(&mut self, class: &str, method: &str, descriptor: &str, kind: HandlerKind) {
        self.handlers
            .insert(make_key(class, method, descriptor), kind);
    }

    /// Register a native method handler.
    pub fn register(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: NativeHandler,
    ) {
        self.insert_handler(class, method, descriptor, HandlerKind::Simple(handler));
    }

    /// Register a native method handler that can call back into the interpreter.
    pub fn register_callback(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: CallbackNativeHandler,
    ) {
        self.insert_handler(class, method, descriptor, HandlerKind::Callback(handler));
    }

    /// Look up a native handler for the given class/method/descriptor.
    ///
    /// Returns `Some` only for `Simple` handlers. Use [`get_kind`] to handle
    /// `Callback` variants.
    ///
    /// [`get_kind`]: NativeRegistry::get_kind
    #[must_use]
    pub fn get(&self, class: &str, method: &str, descriptor: &str) -> Option<NativeHandler> {
        match self.handlers.get(&make_key(class, method, descriptor))? {
            HandlerKind::Simple(h) => Some(*h),
            HandlerKind::Callback(_) => None,
        }
    }

    /// Look up any handler kind for the given class/method/descriptor.
    #[must_use]
    pub fn get_kind(&self, class: &str, method: &str, descriptor: &str) -> Option<HandlerKind> {
        self.handlers
            .get(&make_key(class, method, descriptor))
            .copied()
    }
}

impl Default for NativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
