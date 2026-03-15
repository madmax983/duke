use crate::NativeRegistry;
use crate::build_class_context;
use crate::context::ClassContext;
use duke_loader::ClassLoader;
use duke_runtime::{VmError, VmResult};
use std::collections::{HashMap, HashSet};

/// Registry of loaded classes — maps class name to its ClassContext.
///
/// Used by `execute_class` for cross-class method dispatch.
/// Metadata for a lambda proxy object created by `LambdaMetafactory`.
#[derive(Debug, Clone)]
pub struct LambdaInfo {
    pub impl_class: String,
    pub impl_method: String,
    pub impl_desc: String,
    pub impl_kind: u8,
    pub sam_method: String,
    #[allow(dead_code)]
    pub sam_desc: String,
    pub captured_count: usize,
}

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
    #[must_use]
    pub fn natives(&self) -> &NativeRegistry {
        &self.natives
    }

    /// Access the native method registry mutably.
    pub fn natives_mut(&mut self) -> &mut NativeRegistry {
        &mut self.natives
    }

    /// Register a pre-built ClassContext.
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
    /// loader, parses it, builds a ClassContext, and registers it.
    ///
    /// Also recursively loads the superclass chain so that field slot offsets
    /// can be computed correctly before any object of this type is allocated.
    ///
    /// Returns `Ok(true)` if loaded, `Ok(false)` if the class could not be found
    /// (soft failure — for classes like `java/lang/Object` that we can't load yet).
    pub fn ensure_loaded(&mut self, name: &str, loader: &dyn ClassLoader) -> VmResult<bool> {
        if self.classes.contains_key(name) {
            return Ok(true);
        }
        let bytes = match loader.find_class(name) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };
        let cf = match duke_classfile::parse(&bytes) {
            Ok(cf) => cf,
            Err(_) => return Ok(false),
        };
        let ctx = build_class_context(&cf);
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
