//! Repositories for loaded classes and registered native methods.
//!
//! Provides the execution environment with access to classes, handles class loading and
//! initialization on demand, and maintains the mapping between `native` methods
//! and their Rust implementations.

use std::collections::{HashMap, HashSet};
use std::io::Write;

use duke_loader::ClassLoader;
use duke_runtime::{Slot, VmError, VmResult};

use crate::context::ClassContext;

/// Metadata for a lambda proxy object created by `LambdaMetafactory`.
#[derive(Debug, Clone)]
pub(crate) struct LambdaInfo {
    /// The class name containing the implementation method.
    pub impl_class: String,
    /// The name of the implementation method.
    pub impl_method: String,
    /// The descriptor of the implementation method.
    pub impl_desc: String,
    /// The kind of method handle (e.g., `invokeStatic`, `invokeVirtual`).
    pub impl_kind: u8,
    /// The name of the single abstract method being implemented.
    pub sam_method: String,
    /// The descriptor of the single abstract method being implemented.
    #[allow(dead_code)]
    pub sam_desc: String,
    /// The number of arguments captured by the lambda.
    pub captured_count: usize,
}

/// Threading side-channel requested by a native handler.
///
/// Native handlers run outside the primary JVM event loop. When a native method
/// like `Thread.start0()` or `Thread.sleep()` needs to perform a blocking or
/// thread-lifecycle action, it yields this enum to the interpreter loop, which
/// then manages the actual OS-level thread mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeThreadAction {
    /// Request the VM to spawn a new background thread executing `Thread.run()`.
    Start {
        /// The `u64` reference ID of the `java/lang/Thread` object to start.
        thread_ref: u64
    },
    /// Request the current thread to block for a specified duration.
    Sleep(std::time::Duration),
    /// Request the current thread to wait until another Java thread terminates.
    Join {
        /// The native thread ID (`eetop`) to wait for.
        thread_id: i32
    },
}

/// Per-invocation control state for native handlers.
#[derive(Debug, Default)]
pub struct NativeControl {
    pending_thread_action: Option<NativeThreadAction>,
}

impl NativeControl {
    /// Request a thread-related action from the interpreter loop.
    pub const fn request(&mut self, action: NativeThreadAction) {
        self.pending_thread_action = Some(action);
    }

    /// Take the pending thread-related action, if any.
    pub const fn take(&mut self) -> Option<NativeThreadAction> {
        self.pending_thread_action.take()
    }
}

/// Reflection metadata for one declared method discovered from a classfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectedMethodInfo {
    /// The name of the declared method (e.g., `"hashCode"`).
    pub name: String,
    /// The method's descriptor (e.g., `"()I"`).
    pub descriptor: String,
    /// `true` if the method is marked with `ACC_PUBLIC`.
    pub is_public: bool,
    /// `true` if the method is marked with `ACC_STATIC`.
    pub is_static: bool,
}

/// Reflection metadata for one declared field discovered from a classfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectedFieldInfo {
    /// The name of the declared field (e.g., `"value"`).
    pub name: String,
    /// The field's descriptor (e.g., `"[B"`).
    pub descriptor: String,
    /// `true` if the field is marked with `ACC_PUBLIC`.
    pub is_public: bool,
    /// `true` if the field is marked with `ACC_STATIC`.
    pub is_static: bool,
}

/// Reflection metadata for one class discovered from the loader or registry.
///
/// This provides the necessary information for native reflection methods
/// (like `Class.getDeclaredMethods0`) to inspect a class structure without
/// deeply accessing the internal `ClassContext` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectedClassInfo {
    /// The internal JVM name of the class (e.g., `"java/lang/String"`).
    pub internal_name: String,
    /// The binary name of the class suitable for `Class.getName()` (e.g., `"java.lang.String"`).
    pub binary_name: String,
    /// A list of all methods explicitly declared by this class (excluding inherited ones).
    pub methods: Vec<ReflectedMethodInfo>,
    /// A list of all fields explicitly declared by this class.
    pub fields: Vec<ReflectedFieldInfo>,
}
/// A registry managing loaded classes, their initialization state, and associated native methods.
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
    /// Tracks which classes have had their `<clinit>` run.
    initialized: HashSet<String>,
    /// Lambda proxy class name → metadata.
    lambdas: HashMap<String, LambdaInfo>,
    /// Monotonic counter for generating unique lambda class names.
    lambda_counter: u64,
    #[cfg(feature = "telemetry")]
    pub telemetry: duke_telemetry::TelemetryStore,
}

impl ClassRegistry {
    /// Creates a new empty class registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_interpreter::ClassRegistry;
    ///
    /// let mut registry = ClassRegistry::new();
    /// assert!(!registry.contains("MyClass"));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            natives: NativeRegistry::new(),
            initialized: HashSet::new(),
            lambdas: HashMap::new(),
            lambda_counter: 0,
            #[cfg(feature = "telemetry")]
            telemetry: duke_telemetry::TelemetryStore::default(),
        }
    }

    pub(crate) fn register_lambda(&mut self, info: LambdaInfo) -> String {
        let name = format!("$$Lambda${}", self.lambda_counter);
        self.lambda_counter += 1;
        self.lambdas.insert(name.clone(), info);
        name
    }

    pub(crate) fn get_lambda(&self, class_name: &str) -> Option<&LambdaInfo> {
        self.lambdas.get(class_name)
    }

    /// Check if a class has been initialized (clinit has run).
    #[must_use]
    pub fn is_initialized(&self, name: &str) -> bool {
        self.initialized.contains(name)
    }

    /// Mark a class as initialized.
    pub fn mark_initialized(&mut self, name: &str) {
        self.initialized.insert(name.to_string());
    }

    /// Access the native method registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_interpreter::ClassRegistry;
    ///
    /// let registry = ClassRegistry::new();
    /// assert!(registry.natives().get("java/lang/System", "exit", "(I)V").is_none());
    /// ```
    #[must_use]
    pub const fn natives(&self) -> &NativeRegistry {
        &self.natives
    }

    /// Access the native method registry mutably.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_interpreter::ClassRegistry;
    ///
    /// let mut registry = ClassRegistry::new();
    /// // registry.natives_mut().register(...)
    /// ```
    pub const fn natives_mut(&mut self) -> &mut NativeRegistry {
        &mut self.natives
    }

    /// Register a pre-built `ClassContext`.
    pub fn register(&mut self, ctx: ClassContext) {
        self.classes.insert(ctx.class_name.clone(), ctx);
    }

    /// Get a reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get(&self, name: &str) -> VmResult<&ClassContext> {
        self.classes
            .get(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Get a mutable reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get_mut(&mut self, name: &str) -> VmResult<&mut ClassContext> {
        self.classes
            .get_mut(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Ensure a class is loaded. If not already present, loads it via the class
    /// loader, parses it, builds a `ClassContext`, and registers it.
    ///
    /// Also recursively loads the superclass chain so that field slot offsets
    /// can be computed correctly before any object of this type is allocated.
    ///
    /// Returns `Ok(true)` if loaded, `Ok(false)` if the class could not be found
    /// (soft failure — for classes like `java/lang/Object` that we can't load yet).
    ///
    /// # Errors
    /// Returns [`VmError`] if loading the superclass chain fails unexpectedly.
    pub fn ensure_loaded(&mut self, name: &str, loader: &dyn ClassLoader) -> VmResult<bool> {
        if self.classes.contains_key(name) {
            return Ok(true);
        }
        let Ok(bytes) = loader.find_class(name) else {
            return Ok(false);
        };
        let Ok(cf) = duke_classfile::parse(&bytes) else {
            return Ok(false);
        };
        let ctx = crate::build_class_context(&cf);
        let super_class = ctx.super_class.clone();
        self.classes.insert(name.to_string(), ctx);
        // Recursively load the superclass so ancestor field counts are known.
        if let Some(sc) = super_class {
            self.ensure_loaded(&sc, loader)?;
        }
        Ok(true)
    }

    /// Check if a class is loaded.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Iterate all registered class contexts — used by GC root gathering.
    pub fn all_classes(&self) -> impl Iterator<Item = &ClassContext> {
        self.classes.values()
    }

    /// Mutably iterate all registered class contexts — used to patch static
    /// field slots after a minor GC collection.
    pub fn all_classes_mut(&mut self) -> impl Iterator<Item = &mut ClassContext> {
        self.classes.values_mut()
    }
}

impl Default for ClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Signature for native method implementations.
///
/// Arguments:
/// - `&[Slot]`: method arguments (including `this` in slot 0 for instance methods)
/// - `&mut Heap`: the object heap for reading/writing objects
/// - `&mut dyn Write`: output sink (stdout in production, `Vec<u8>` in tests)
/// - `&mut NativeControl`: side channel for blocking/thread actions
pub type NativeHandler =
    fn(&[Slot], &mut duke_gc::Heap, &mut dyn Write, &mut NativeControl) -> VmResult<Option<Slot>>;

/// Mutable helper surface exposed to callback natives.
///
/// This keeps class loading, reflection metadata inspection, and nested Java
/// invocation behind one mutable object so natives do not need direct access to
/// the interpreter's registry/loader state.
pub trait CallbackOps {
    /// Invoke a Java method through the current interpreter/runtime boundary.
    ///
    /// # Errors
    /// Returns any VM error produced while resolving or executing the target method.
    fn invoke(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
        method: &str,
        descriptor: &str,
        args: Vec<Slot>,
    ) -> VmResult<Option<Slot>>;

    /// Ensure the named class is available to the current runtime.
    ///
    /// # Errors
    /// Returns an error if the class cannot be loaded or linked.
    fn ensure_loaded(&mut self, class: &str) -> VmResult<()>;

    /// Read reflection metadata for a loaded or loadable class.
    ///
    /// # Errors
    /// Returns an error if the class cannot be inspected.
    fn inspect_class(&mut self, class: &str) -> VmResult<ReflectedClassInfo>;
}

/// A native handler that can call back into the interpreter to invoke Java methods
/// and query reflection metadata from the current loader/registry.
pub type CallbackNativeHandler = fn(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>>;

/// Stored in `NativeRegistry` — all existing handlers stay `Simple`.
#[derive(Copy, Clone, Debug)]
pub enum HandlerKind {
    /// A simple stateless native method handler.
    Simple(NativeHandler),
    /// A handler that delegates back to a callback trait or closure.
    Callback(CallbackNativeHandler),
}

/// Registry of native method implementations.
///
/// Maps `"class_name\x00method_name\x00descriptor"` to a handler kind.
///
/// # Examples
///
/// ```
/// use duke_interpreter::NativeRegistry;
///
/// let mut natives = NativeRegistry::new();
/// assert!(natives.get("java/lang/System", "exit", "(I)V").is_none());
/// ```
pub struct NativeRegistry {
    handlers: HashMap<String, HandlerKind>,
}

/// Build the lookup key for a native method: `"class\x00method\x00desc"`.
///
/// Using NUL as separator avoids ambiguity (JVM identifiers cannot contain NUL)
/// and reduces the three allocations previously required per lookup to one.
#[inline]
fn make_key(class: &str, method: &str, desc: &str) -> String {
    let mut key = String::with_capacity(class.len() + method.len() + desc.len() + 2);
    key.push_str(class);
    key.push('\x00');
    key.push_str(method);
    key.push('\x00');
    key.push_str(desc);
    key
}

impl NativeRegistry {
    /// Creates a new empty native registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_interpreter::NativeRegistry;
    ///
    /// let natives = NativeRegistry::new();
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_registry_default() {
        let registry = ClassRegistry::default();
        assert!(!registry.contains("java/lang/Object"));
    }

    #[test]
    fn test_native_registry_default() {
        let natives = NativeRegistry::default();
        assert!(natives.get("java/lang/System", "exit", "(I)V").is_none());
    }
}
