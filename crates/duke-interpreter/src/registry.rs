//! Repositories for loaded classes and registered native methods.
//!
//! Provides the execution environment with access to classes, handles class loading and
//! initialization on demand, and maintains the mapping between `native` methods
//! and their Rust implementations.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use duke_loader::ClassLoader;
use duke_runtime::{Slot, VmError, VmResult};

use crate::context::ClassContext;

fn class_internal_name_fragment(name: &str) -> &str {
    name.split_once('\0')
        .map_or(name, |(internal_name, _)| internal_name)
}

/// Metadata for a lambda proxy object created by `LambdaMetafactory`.
#[derive(Debug, Clone)]
pub(crate) struct LambdaInfo {
    pub impl_class: String,
    pub impl_method: String,
    pub impl_desc: String,
    pub impl_kind: u8,
    pub sam_method: String,
    #[allow(dead_code)]
    pub sam_desc: String,
    pub captured_count: usize,
}

/// Threading side-channel requested by a native handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeThreadAction {
    Start { thread_ref: u64 },
    Sleep(std::time::Duration),
    Join { thread_id: i32 },
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
    pub name: String,
    pub descriptor: String,
    pub is_public: bool,
    pub is_static: bool,
}

/// Reflection metadata for one declared field discovered from a classfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectedFieldInfo {
    pub name: String,
    pub descriptor: String,
    pub is_public: bool,
    pub is_static: bool,
}

/// Reflection metadata for one class discovered from the loader or registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectedClassInfo {
    pub internal_name: String,
    pub binary_name: String,
    pub super_class: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<ReflectedMethodInfo>,
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
    /// Default code source path used for lightweight `ProtectionDomain` emulation.
    default_code_source: Option<String>,
    /// ZIP/JAR-backed class loaders keyed by their stable archive path.
    archive_loaders: HashMap<String, Arc<dyn ClassLoader + Send + Sync>>,
    /// Best-known code source path for each loaded class.
    class_code_sources: HashMap<String, String>,
    /// Best-known runtime `java/lang/ClassLoader` object for each loaded class.
    class_runtime_loaders: HashMap<String, u64>,
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
            default_code_source: None,
            archive_loaders: HashMap::new(),
            class_code_sources: HashMap::new(),
            class_runtime_loaders: HashMap::new(),
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

    /// Set the default code source path used when Java code asks for a class's
    /// protection domain before Duke has full per-class provenance tracking.
    pub fn set_default_code_source(&mut self, path: impl Into<String>) {
        self.default_code_source = Some(path.into());
    }

    /// Read the configured default code source path, if any.
    #[must_use]
    pub fn default_code_source(&self) -> Option<&str> {
        self.default_code_source.as_deref()
    }

    /// Return the best-known code source path for `class`, if Duke has one.
    #[must_use]
    pub fn code_source_for_class(&self, class: &str) -> Option<&str> {
        self.class_code_sources
            .get(class)
            .map(String::as_str)
            .or_else(|| self.default_code_source())
    }

    /// Return the cached defining loader for `class`, if any.
    #[must_use]
    pub fn class_loader(&self, class: &str) -> Option<&Arc<dyn ClassLoader + Send + Sync>> {
        self.class_code_sources
            .get(class)
            .and_then(|path| self.archive_loaders.get(path))
    }

    /// Return the best-known runtime `java/lang/ClassLoader` object for `class`.
    #[must_use]
    pub fn runtime_loader_for_class(&self, class: &str) -> Option<u64> {
        self.class_runtime_loaders.get(class).copied()
    }

    /// Return the human-facing internal name for `class`, stripping any loader provenance.
    #[must_use]
    pub fn internal_name_for_class<'a>(&self, class: &'a str) -> &'a str {
        class_internal_name_fragment(class)
    }

    fn is_plain_bootstrap_class(&self, class: &str) -> bool {
        self.classes.contains_key(class)
            && !self.class_code_sources.contains_key(class)
            && !self.class_runtime_loaders.contains_key(class)
    }

    /// Compute the deterministic class identity key for a class reference under explicit provenance.
    #[must_use]
    pub fn class_key_from_provenance(
        &self,
        internal_name: &str,
        code_source: Option<&str>,
        runtime_loader: Option<u64>,
    ) -> String {
        let internal_name = class_internal_name_fragment(internal_name);
        if internal_name.starts_with('[')
            || internal_name.len() == 1
            || self.is_plain_bootstrap_class(internal_name)
        {
            return internal_name.to_string();
        }
        if let Some(loader_ref) = runtime_loader {
            return format!("{internal_name}\0loader:{loader_ref}");
        }
        if let Some(path) = code_source {
            return format!("{internal_name}\0code:{path}");
        }
        internal_name.to_string()
    }

    /// Compute the class identity key implied by the provenance of `source_class`.
    #[must_use]
    pub fn class_key_from_source(&self, internal_name: &str, source_class: Option<&str>) -> String {
        let code_source = source_class.and_then(|class| self.explicit_code_source_for_class(class));
        let runtime_loader = source_class.and_then(|class| self.runtime_loader_for_class(class));
        self.class_key_from_provenance(internal_name, code_source, runtime_loader)
    }

    #[must_use]
    fn explicit_code_source_for_class(&self, class: &str) -> Option<&str> {
        self.class_code_sources.get(class).map(String::as_str)
    }

    fn zip_loader_for_path(&mut self, path: &str) -> Option<Arc<dyn ClassLoader + Send + Sync>> {
        if let Some(loader) = self.archive_loaders.get(path) {
            return Some(Arc::clone(loader));
        }
        let loader = duke_loader::ZipLoader::open(Path::new(path)).ok()?;
        let loader: Arc<dyn ClassLoader + Send + Sync> = Arc::new(loader);
        self.archive_loaders
            .insert(path.to_string(), Arc::clone(&loader));
        Some(loader)
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
        self.ensure_loaded_inner(name, loader, None, None)
    }

    /// Ensure a class is loaded using the same archive/classpath provenance as `source_class`.
    ///
    /// # Errors
    /// Returns [`VmError`] if the selected loader fails to parse, link, or resolve the requested class.
    pub fn ensure_loaded_from(
        &mut self,
        name: &str,
        source_class: Option<&str>,
        fallback_loader: &dyn ClassLoader,
    ) -> VmResult<bool> {
        if let Some(source_class) = source_class
            && let Some(path) = self
                .explicit_code_source_for_class(source_class)
                .map(ToOwned::to_owned)
        {
            return self.ensure_loaded_with_provenance(
                name,
                &path,
                self.runtime_loader_for_class(source_class),
            );
        }
        self.ensure_loaded(name, fallback_loader)
    }

    /// Ensure a class is loaded from a specific archive path and record that provenance.
    ///
    /// # Errors
    /// Returns [`VmError`] if the selected archive loader fails to parse, link, or resolve the requested class.
    pub fn ensure_loaded_with_code_source(&mut self, name: &str, path: &str) -> VmResult<bool> {
        self.ensure_loaded_with_provenance(name, path, None)
    }

    /// Ensure a class is loaded from a specific archive path and runtime loader.
    ///
    /// # Errors
    /// Returns [`VmError`] if the selected archive loader fails to parse, link, or resolve the requested class.
    pub fn ensure_loaded_with_provenance(
        &mut self,
        name: &str,
        path: &str,
        runtime_loader: Option<u64>,
    ) -> VmResult<bool> {
        let Some(loader) = self.zip_loader_for_path(path) else {
            return Ok(false);
        };
        self.ensure_loaded_inner(name, loader.as_ref(), Some(path), runtime_loader)
    }

    fn ensure_loaded_inner(
        &mut self,
        name: &str,
        loader: &dyn ClassLoader,
        code_source: Option<&str>,
        runtime_loader: Option<u64>,
    ) -> VmResult<bool> {
        let internal_name = class_internal_name_fragment(name).to_string();
        let class_key = self.class_key_from_provenance(&internal_name, code_source, runtime_loader);
        if self.classes.contains_key(&class_key) {
            if let Some(path) = code_source {
                self.class_code_sources
                    .entry(class_key.clone())
                    .or_insert_with(|| path.to_string());
            }
            if let Some(loader_ref) = runtime_loader {
                self.class_runtime_loaders
                    .entry(class_key)
                    .or_insert(loader_ref);
            }
            return Ok(true);
        }
        let Ok(bytes) = loader.find_class(&internal_name) else {
            return Ok(false);
        };
        let Ok(cf) = duke_classfile::parse(&bytes) else {
            return Ok(false);
        };
        let mut ctx = crate::build_class_context(&cf);
        let resolved_super_class = if let Some(super_class) = ctx.super_class.clone() {
            let _ = self.ensure_loaded_inner(&super_class, loader, code_source, runtime_loader)?;
            Some(self.class_key_from_provenance(&super_class, code_source, runtime_loader))
        } else {
            None
        };
        let mut resolved_interfaces = Vec::with_capacity(ctx.interfaces.len());
        for interface in ctx.interfaces.clone() {
            let _ = self.ensure_loaded_inner(&interface, loader, code_source, runtime_loader)?;
            resolved_interfaces.push(self.class_key_from_provenance(
                &interface,
                code_source,
                runtime_loader,
            ));
        }
        ctx.class_name.clone_from(&class_key);
        ctx.super_class = resolved_super_class;
        ctx.interfaces = resolved_interfaces;
        self.classes.insert(class_key.clone(), ctx);
        if let Some(path) = code_source {
            self.class_code_sources
                .insert(class_key.clone(), path.to_string());
        }
        if let Some(loader_ref) = runtime_loader {
            self.class_runtime_loaders.insert(class_key, loader_ref);
        }
        Ok(true)
    }

    /// Check if a class is loaded.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Resolve `name` to an exact loaded class key.
    ///
    /// Accepts either an exact class key or a plain internal name when exactly one
    /// loaded class matches that internal name.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] when no loaded class matches, or
    /// [`VmError::AmbiguousClassName`] when multiple loaded classes share the same
    /// internal name.
    pub fn resolve_loaded_class_key(&self, name: &str) -> VmResult<String> {
        if self.classes.contains_key(name) {
            return Ok(name.to_string());
        }
        let internal_name = self.internal_name_for_class(name);
        let matches: Vec<String> = self
            .classes
            .keys()
            .filter(|class| self.internal_name_for_class(class) == internal_name)
            .cloned()
            .collect();
        match matches.as_slice() {
            [] => Err(VmError::ClassNotFound {
                name: name.to_string(),
            }),
            [only] => Ok(only.clone()),
            _ => Err(VmError::AmbiguousClassName {
                name: internal_name.to_string(),
                matches,
            }),
        }
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

    /// Ensure the named class has completed initialization, including `<clinit>`.
    ///
    /// # Errors
    /// Returns an error if the class cannot be loaded or if initialization fails.
    fn ensure_class_initialized(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        class: &str,
    ) -> VmResult<()> {
        self.ensure_loaded(class)
    }

    /// Return the best-known code source path for the given class.
    ///
    /// Duke currently uses a coarse default path while bootstrapping richer
    /// class provenance support.
    ///
    /// # Errors
    /// Returns an error if class provenance lookup fails.
    fn code_source_for_class(&mut self, _class: &str) -> VmResult<Option<String>> {
        Ok(None)
    }

    /// Load `class` using the specific runtime `ClassLoader` object identified by `loader_ref`.
    ///
    /// # Errors
    /// Returns an error if the class cannot be loaded or linked under that runtime loader.
    fn ensure_loaded_with_runtime_loader(
        &mut self,
        _heap: &duke_gc::Heap,
        _loader_ref: u64,
        class: &str,
    ) -> VmResult<()> {
        self.ensure_loaded(class)
    }

    /// Resolve an instance field slot for the named class.
    ///
    /// # Errors
    /// Returns an error if the field cannot be resolved.
    fn instance_field_slot(&mut self, _class: &str, _field_name: &str) -> VmResult<usize> {
        Err(VmError::InvalidFieldref { index: 0 })
    }

    /// Read an instance field from `object_ref`, validating it against the declaring class.
    ///
    /// # Errors
    /// Returns an error if the field cannot be resolved or read.
    fn read_instance_field(
        &mut self,
        _heap: &duke_gc::Heap,
        _object_ref: u64,
        _declaring_class: &str,
        _field_name: &str,
    ) -> VmResult<Slot> {
        Err(VmError::InvalidFieldref { index: 0 })
    }

    /// Write an instance field on `object_ref`, validating it against the declaring class.
    ///
    /// # Errors
    /// Returns an error if the field cannot be resolved or written.
    fn write_instance_field(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _object_ref: u64,
        _declaring_class: &str,
        _field_name: &str,
        _value: Slot,
    ) -> VmResult<()> {
        Err(VmError::InvalidFieldref { index: 0 })
    }

    /// Read a static field from the named class.
    ///
    /// # Errors
    /// Returns an error if the field cannot be resolved or read.
    fn read_static_field(&mut self, _class: &str, _field_name: &str) -> VmResult<Slot> {
        Err(VmError::InvalidFieldref { index: 0 })
    }

    /// Write a static field on the named class.
    ///
    /// # Errors
    /// Returns an error if the field cannot be resolved or written.
    fn write_static_field(
        &mut self,
        _class: &str,
        _field_name: &str,
        _value: Slot,
    ) -> VmResult<()> {
        Err(VmError::InvalidFieldref { index: 0 })
    }

    /// Return the runtime `java/lang/ClassLoader` object for `class`, if known.
    ///
    /// # Errors
    /// Returns an error if runtime loader provenance lookup fails.
    fn runtime_loader_for_class(&mut self, _class: &str) -> VmResult<Option<u64>> {
        Ok(None)
    }

    /// Return the deterministic class identity key for `class` in the current runtime.
    ///
    /// # Errors
    /// Returns an error if class identity resolution fails.
    fn class_key_for_loaded_class(&mut self, class: &str) -> VmResult<String> {
        Ok(class.to_string())
    }

    /// Return the deterministic class identity key for `class` under the given runtime loader.
    ///
    /// # Errors
    /// Returns an error if class identity resolution fails under that loader.
    fn class_key_for_runtime_loader(
        &mut self,
        _heap: &duke_gc::Heap,
        _loader_ref: u64,
        class: &str,
    ) -> VmResult<String> {
        self.class_key_for_loaded_class(class)
    }

    /// Return the deterministic class identity key for `class` as seen from `source_class`.
    ///
    /// # Errors
    /// Returns an error if class identity resolution fails for the source provenance.
    fn class_key_from_source(
        &mut self,
        class: &str,
        _source_class: Option<&str>,
    ) -> VmResult<String> {
        self.class_key_for_loaded_class(class)
    }

    /// Allocate a new heap instance for the named class, applying normal class initialization.
    ///
    /// # Errors
    /// Returns an error if allocation or class initialization fails.
    fn allocate_instance(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
    ) -> VmResult<u64> {
        Err(VmError::Unimplemented {
            mnemonic: "reflection constructor allocation",
        })
    }
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
    use std::path::PathBuf;

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

    #[test]
    fn ensure_loaded_with_code_source_uses_boot_archive_layout() {
        let jar_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hello.jar");
        let mut registry = ClassRegistry::new();
        assert!(
            registry
                .ensure_loaded_with_code_source("HelloWorld", &jar_path.display().to_string())
                .expect("load HelloWorld from code source"),
            "zip-backed code source should resolve classes"
        );
        let class_key = registry.class_key_from_provenance(
            "HelloWorld",
            Some(&jar_path.display().to_string()),
            None,
        );
        assert!(registry.contains(&class_key));
        assert_eq!(
            registry.code_source_for_class(&class_key),
            Some(jar_path.display().to_string().as_str())
        );
    }
}
