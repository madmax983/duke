//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

pub mod native;
pub use native::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};

/// A decoded method ready for execution.
pub struct MethodEntry {
    pub name: String,
    pub descriptor: String,
    pub instructions: Vec<(usize, Instruction)>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub exception_table: Vec<ExceptionEntry>,
    /// Precomputed PC → instruction-index map, shared cheaply via Arc.
    pub pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
}

/// A field declaration extracted from a parsed class.
pub struct FieldEntry {
    pub name: String,
    pub descriptor: String,
    /// True if declared `static`.
    pub is_static: bool,
}

/// A resolved exception table entry for handler dispatch.
///
/// Built from `duke_classfile::types::ExceptionTableEntry` with catch_type
/// resolved from a CP index to a class name string.
pub struct ExceptionEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// `None` for catch-all (finally). `Some(class_name)` for typed catches.
    pub catch_type: Option<String>,
}

/// A parsed class with all methods decoded — the unit of execution for Phase 5+.
pub struct ClassContext {
    /// Internal JVM class name (e.g. `"Point"`).
    pub class_name: String,
    /// Superclass name (`None` for `java/lang/Object`).
    pub super_class: Option<String>,
    /// Directly implemented interfaces (used by checkcast / instanceof).
    pub interfaces: Vec<String>,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    /// All field declarations (static and instance), in class file order.
    pub fields: Vec<FieldEntry>,
    /// Values of static fields, indexed by position among static-only fields.
    pub static_fields: Vec<Slot>,
    /// Number of instance (non-static) fields — used to size heap objects at `new`.
    pub instance_field_count: usize,
    /// BootstrapMethods entries from the class attribute (needed for invokedynamic).
    pub bootstrap_methods: Vec<duke_classfile::types::BootstrapMethodEntry>,
}

/// Registry of loaded classes — maps class name to its ClassContext.
///
/// Used by `execute_class` for cross-class method dispatch.
/// Metadata for a lambda proxy object created by `LambdaMetafactory`.
#[derive(Debug, Clone)]
struct LambdaInfo {
    impl_class: String,
    impl_method: String,
    impl_desc: String,
    impl_kind: u8,
    sam_method: String,
    #[allow(dead_code)]
    sam_desc: String,
    captured_count: usize,
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

    fn register_lambda(&mut self, info: LambdaInfo) -> String {
        let name = format!("$$Lambda${}", self.lambda_counter);
        self.lambda_counter += 1;
        self.lambdas.insert(name, info);
        name
    }

    fn get_lambda(&self, class_name: &str) -> Option<&LambdaInfo> {
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
        self.classes.insert(ctx.class_name, ctx);
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
        let super_class = ctx.super_class;
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

pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution.
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    // Local heap for array objects allocated during single-method execution.
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            // ----------------------------------------------------------------
            // Constants
            // ----------------------------------------------------------------
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

            // ----------------------------------------------------------------
            // Constant pool load
            // ----------------------------------------------------------------
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(cp_idx).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s,
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(idx_val).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s,
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
                }
            }

            // ----------------------------------------------------------------
            // Loads
            // ----------------------------------------------------------------
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }

            // ----------------------------------------------------------------
            // Stores
            // ----------------------------------------------------------------
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }

            // ----------------------------------------------------------------
            // Stack manipulation
            // ----------------------------------------------------------------
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }

            // ----------------------------------------------------------------
            // Integer arithmetic
            // ----------------------------------------------------------------
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }

            // ----------------------------------------------------------------
            // Long arithmetic
            // ----------------------------------------------------------------
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Float arithmetic
            // ----------------------------------------------------------------
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1 // NaN result: Fcmpg pushes 1, Fcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Double arithmetic
            // ----------------------------------------------------------------
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1 // NaN result: Dcmpg pushes 1, Dcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Type conversions
            // ----------------------------------------------------------------
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i8 as i32))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as u16 as i32))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i16 as i32))?;
            }

            // ----------------------------------------------------------------
            // Returns
            // ----------------------------------------------------------------
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),

            // ----------------------------------------------------------------
            // Branches
            // ----------------------------------------------------------------
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),

            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            // Reference comparisons
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let init_slot = match array_type {
                    ArrayType::Long => Slot::Long(0),
                    ArrayType::Float => Slot::Float(0.0),
                    ArrayType::Double => Slot::Double(0.0),
                    _ => Slot::Int(0),
                };
                let r = local_heap.len() as u64;
                local_heap.push((
                    class_name.to_string(),
                    vec![init_slot; count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = resolve_class_name(cp, usize::from(cp_idx.0))?;
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = local_heap.len() as u64;
                local_heap.push((
                    array_type,
                    vec![Slot::Reference(None); count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()?;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_long()?;
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_float()?;
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_double()?;
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize];
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()? as i8 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = frame.pop_int()? as i8 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()? as u16 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = frame.pop_int()? as u16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()? as i16 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = frame.pop_int()? as i16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            ;
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            ;
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow ----
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .0
                    ;
                return Err(VmError::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter => {
                let _obj = frame.pop()?;
            }
            Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        idx += 1;
    }
}

/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    triggered_by: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);

    // Check if the class has a <clinit> method.
    let has_clinit = {
        match registry.get(class_name) {
            Ok(ctx) => ctx
                .methods
                .iter()
                .any(|m| m.name == "<clinit>" && m.descriptor == "()V"),
            Err(_) => false,
        }
    };

    if has_clinit {
        // Run <clinit> by calling it through execute_class.
        #[cfg(feature = "telemetry")]
        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth calls; all subsequent frames are pool hits with zero allocation.
///
/// The pool is private to a single `execute_class` invocation.
struct FramePool {
    free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair.
    /// Returns a pooled pair if available, otherwise allocates fresh Vecs.
    fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    /// `stack` must already be empty (guaranteed by `Frame::into_pool_bufs`).
    /// Caps pool at 256 entries to bound memory usage.
    fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        // Cap at 256 entries — typical max call depth is well under 100; anything
        // beyond this is dead weight. Entries dropped here are freed to the allocator.
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}

#[cfg(feature = "telemetry")]
fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 | I::IstoreW(_) => {
            "istore"
        }
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 | I::LstoreW(_) => {
            "lstore"
        }
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 | I::FstoreW(_) => {
            "fstore"
        }
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 | I::DstoreW(_) => {
            "dstore"
        }
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 | I::AstoreW(_) => {
            "astore"
        }
        I::Iaload => "iaload",
        I::Laload => "laload",
        I::Faload => "faload",
        I::Daload => "daload",
        I::Aaload => "aaload",
        I::Baload => "baload",
        I::Caload => "caload",
        I::Saload => "saload",
        I::Iastore => "iastore",
        I::Lastore => "lastore",
        I::Fastore => "fastore",
        I::Dastore => "dastore",
        I::Aastore => "aastore",
        I::Bastore => "bastore",
        I::Castore => "castore",
        I::Sastore => "sastore",
        I::Pop => "pop",
        I::Pop2 => "pop2",
        I::Dup => "dup",
        I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2",
        I::Dup2 => "dup2",
        I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2",
        I::Swap => "swap",
        I::Iadd => "iadd",
        I::Ladd => "ladd",
        I::Fadd => "fadd",
        I::Dadd => "dadd",
        I::Isub => "isub",
        I::Lsub => "lsub",
        I::Fsub => "fsub",
        I::Dsub => "dsub",
        I::Imul => "imul",
        I::Lmul => "lmul",
        I::Fmul => "fmul",
        I::Dmul => "dmul",
        I::Idiv => "idiv",
        I::Ldiv => "ldiv",
        I::Fdiv => "fdiv",
        I::Ddiv => "ddiv",
        I::Irem => "irem",
        I::Lrem => "lrem",
        I::Frem => "frem",
        I::Drem => "drem",
        I::Ineg => "ineg",
        I::Lneg => "lneg",
        I::Fneg => "fneg",
        I::Dneg => "dneg",
        I::Ishl => "ishl",
        I::Lshl => "lshl",
        I::Ishr => "ishr",
        I::Lshr => "lshr",
        I::Iushr => "iushr",
        I::Lushr => "lushr",
        I::Iand => "iand",
        I::Land => "land",
        I::Ior => "ior",
        I::Lor => "lor",
        I::Ixor => "ixor",
        I::Lxor => "lxor",
        I::Iinc { .. } | I::IincW { .. } => "iinc",
        I::I2l => "i2l",
        I::I2f => "i2f",
        I::I2d => "i2d",
        I::L2i => "l2i",
        I::L2f => "l2f",
        I::L2d => "l2d",
        I::F2i => "f2i",
        I::F2l => "f2l",
        I::F2d => "f2d",
        I::D2i => "d2i",
        I::D2l => "d2l",
        I::D2f => "d2f",
        I::I2b => "i2b",
        I::I2c => "i2c",
        I::I2s => "i2s",
        I::Lcmp => "lcmp",
        I::Fcmpl => "fcmpl",
        I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl",
        I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) | I::RetW(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn",
        I::Lreturn => "lreturn",
        I::Freturn => "freturn",
        I::Dreturn => "dreturn",
        I::Areturn => "areturn",
        I::Return => "return",
        I::Getstatic(_) => "getstatic",
        I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield",
        I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface { .. } => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray",
        I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast",
        I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter",
        I::Monitorexit => "monitorexit",
        I::Multianewarray { .. } => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
    }
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match registry
        .natives
        .get_kind(class_name, method_name, descriptor)
    {
        Some(HandlerKind::Simple(h)) => {
            return h(args, heap, stdout);
        }
        Some(HandlerKind::Callback(h)) => {
            // TODO: this `invoke_cb` closure body is duplicated at 5 dispatch
            // sites. It can't be extracted into a free function because it
            // captures `registry` and `loader` from the enclosing frame; a
            // macro or an `InvokeContext` struct would remove the duplication.
            let mut invoke_cb = |heap: &mut duke_gc::Heap,
                                 output: &mut dyn std::io::Write,
                                 class: &str,
                                 method: &str,
                                 desc: &str,
                                 cb_args: Vec<Slot>|
             -> VmResult<Option<Slot>> {
                execute_class(
                    registry, loader, heap, output, class, method, desc, &cb_args,
                )
            };
            return h(args, heap, stdout, &mut invoke_cb);
        }
        None => {}
    }

    // Find entry method.
    let entry_idx = {
        let ctx = registry.get(class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| VmError::MethodNotFound {
                name: method_name.to_string(),
                descriptor: descriptor.to_string(),
            })?
    };

    // Ensure the entry class has been initialized (<clinit> run).
    ensure_initialized(registry, loader, heap, stdout, class_name, "")?;

    let mut current_class = class_name.to_string();
    #[cfg(feature = "telemetry")]
    let mut current_method = method_name.to_string();
    let mut call_stack: Vec<CallFrame> = Vec::new();
    let mut frame_pool = FramePool::new();
    // Dispatch cache: caller_class_name -> cp_idx -> (callee_class_name, method_idx, arg_count).
    // Nested map lets the outer lookup borrow current_class as &str (no clone on cache hits).
    // Eliminates repeated CP 3-level walk + linear method search for repeat static/special call sites.
    let mut dispatch_cache: HashMap<String, HashMap<u16, (String, usize, usize)>> = HashMap::new();
    let mut method_idx = entry_idx;
    let mut pc_to_idx = {
        let ctx = registry.get(&current_class)?;
        std::sync::Arc::clone(&ctx.methods[method_idx].pc_to_idx)
    };
    let mut frame = {
        let ctx = registry.get(&current_class)?;
        Frame::new(
            usize::from(ctx.methods[method_idx].max_stack),
            usize::from(ctx.methods[method_idx].max_locals),
            args.to_vec(),
        )?
    };
    let mut idx: usize = 0;
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    loop {
        let (pc, instr) = {
            let ctx = registry.get(&current_class)?;
            let Some(&(pc, ref instr)) = ctx.methods[method_idx].instructions.get(idx) else {
                return Err(VmError::FellOffEnd);
            };
            (pc, instr)
        };

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        // Helper macro for return instructions: pop call stack or return to Rust.
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok($val),
                    Some(caller) => {
                        let ret_val = $val;
                        // Recycle callee frame buffers before overwriting `frame`.
                        let old = std::mem::replace(&mut frame, caller.frame);
                        let (l, s) = old.into_pool_bufs();
                        frame_pool.release(l, s);
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        current_class = caller.class_name;
                        #[cfg(feature = "telemetry")]
                        {
                            current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(method_idx)
                                        .map(|m| m.name)
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }

        // Telemetry: capture opcode name and start time before dispatch.
        // Arms that use `continue` (branches, invokes) will skip the post-match
        // recording for that iteration — timing is approximate for those opcodes.
        #[cfg(feature = "telemetry")]
        let (_telem_name, _telem_pc, _telem_start) = {
            let name = instr_name(&instr);
            let pc_val = pc;
            (name, pc_val, std::time::Instant::now())
        };

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                if let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                    .get(current_class.as_str())
                    .and_then(|m| m.get(&cp_idx.0))
                {
                    // Fast path: cache hit — skip CP walk and method search.
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac,
                                max_locals,
                            });
                        }
                        for i in (0..cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let callee_class = cached_cls;
                    let callee_idx = cached_idx;
                    call_stack.push(CallFrame {
                        frame,
                        method_idx,
                        pc_to_idx,
                        resume_idx: idx + 1,
                        class_name: current_class,
                    });
                    frame = callee_frame;
                    method_idx = callee_idx;
                    pc_to_idx = callee_pc_to_idx;
                    current_class = callee_class;
                    #[cfg(feature = "telemetry")]
                    {
                        current_method = registry
                            .get(&current_class)
                            .map(|c| {
                                c.methods
                                    .get(method_idx)
                                    .map(|m| m.name)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                    }
                    idx = 0;
                    continue;
                }
                // Slow path: full CP resolution + method search.
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&callee_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &callee_class,
                    &current_class,
                )?;
                let callee_idx = {
                    let ctx = registry.get(&callee_class)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        dispatch_cache
                            .entry(current_class)
                            .or_default()
                            .insert(cp_idx.0, (callee_class, callee_idx, arg_count));
                        let (callee_pc_to_idx, callee_frame) = {
                            let ctx = registry.get(&callee_class)?;
                            let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                            let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                            let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                            locals_buf.resize(max_locals, Slot::Int(0));
                            if arg_count > max_locals {
                                return Err(VmError::LocalOutOfBounds {
                                    index: arg_count,
                                    max_locals,
                                });
                            }
                            for i in (0..arg_count).rev() {
                                locals_buf[i] = frame.pop()?;
                            }
                            let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                            (pci, f)
                        };
                        call_stack.push(CallFrame {
                            frame,
                            method_idx,
                            pc_to_idx,
                            resume_idx: idx + 1,
                            class_name: current_class,
                        });
                        frame = callee_frame;
                        method_idx = callee_idx;
                        pc_to_idx = callee_pc_to_idx;
                        current_class = callee_class;
                        #[cfg(feature = "telemetry")]
                        {
                            current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(method_idx)
                                        .map(|m| m.name)
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        let handler_kind =
                            registry
                                .natives
                                .get_kind(&callee_class, &callee_name, &callee_desc);
                        match handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                native_args.reverse();
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let result = handler(&native_args, heap, stdout);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = result?;
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                native_args.reverse();
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut invoke_cb =
                                    |heap: &mut duke_gc::Heap,
                                     output: &mut dyn std::io::Write,
                                     class: &str,
                                     method: &str,
                                     desc: &str,
                                     cb_args: Vec<Slot>|
                                     -> VmResult<Option<Slot>> {
                                        execute_class(
                                            registry, loader, heap, output, class, method, desc,
                                            &cb_args,
                                        )
                                    };
                                let result = handler(&native_args, heap, stdout, &mut invoke_cb);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = result?;
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            None => {
                                return Err(VmError::MethodNotFound {
                                    name: callee_name,
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                }
            }

            // ---- returns ----
            Instruction::Return => do_return!(None),
            Instruction::Ireturn => {
                let v = frame.pop_int()?;
                do_return!(Some(Slot::Int(v)));
            }
            Instruction::Lreturn => {
                let v = frame.pop_long()?;
                do_return!(Some(Slot::Long(v)));
            }
            Instruction::Freturn => {
                let v = frame.pop_float()?;
                do_return!(Some(Slot::Float(v)));
            }
            Instruction::Dreturn => {
                let v = frame.pop_double()?;
                do_return!(Some(Slot::Double(v)));
            }

            // ---- all other instructions: same as execute() ----
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                let string_info = {
                    let ctx = registry.get(&current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s,
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(&current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = cp_idx + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(&current_class)?;
                        ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
                    }
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                let string_info = {
                    let ctx = registry.get(&current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s,
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(&current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = idx_val + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(&current_class)?;
                        ldc_push(&mut frame, &ctx.constant_pool, idx_val)?;
                    }
                }
            }
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i8 as i32))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as u16 as i32))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i16 as i32))?;
            }
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),
            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ---- Object allocation ----
            Instruction::New(cp_idx) => {
                let target_class = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                // Walk the super chain to sum all instance field counts
                // (e.g. Enum has 2 fields inherited by every enum subclass).
                let field_count = {
                    let mut count = registry
                        .get(&target_class)
                        .map(|c| c.instance_field_count)
                        .unwrap_or(0);
                    let mut sc = registry
                        .get(&target_class)
                        .ok()
                        .and_then(|c| c.super_class);
                    while let Some(ref s) = sc {
                        match registry.get(s) {
                            Ok(sctx) => {
                                count += sctx.instance_field_count;
                                sc = sctx.super_class;
                            }
                            Err(_) => break,
                        }
                    }
                    count
                };
                #[cfg(feature = "telemetry")]
                registry.telemetry.object_lineage.record(
                    &current_class,
                    &current_method,
                    pc,
                    &target_class,
                );
                let r = heap.allocate(target_class, field_count);
                // Set reference/long/float/double fields to their JVM-spec defaults.
                // heap.allocate initialises everything to Int(0), which is wrong
                // for reference-typed fields (should be Reference(None)).
                init_object_fields(registry, heap, r, &target_class);
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                let val = heap.get(r)?.fields[fidx];
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                heap.write_field(r, fidx, val)?;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                let val = registry.get(&target_class)?.static_fields[sidx];
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                registry.get_mut(&target_class)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                // Fast path: cache hit for invokespecial (static dispatch — safe to cache).
                if matches!(instr, Instruction::Invokespecial(_))
                    && let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                {
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac + 1 > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac + 1,
                                max_locals,
                            });
                        }
                        for i in (1..=cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        locals_buf[0] = frame.pop()?; // `this`
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let dispatch_class = cached_cls;
                    let callee_idx = cached_idx;
                    call_stack.push(CallFrame {
                        frame,
                        method_idx,
                        pc_to_idx,
                        resume_idx: idx + 1,
                        class_name: current_class,
                    });
                    frame = callee_frame;
                    method_idx = callee_idx;
                    pc_to_idx = callee_pc_to_idx;
                    current_class = dispatch_class;
                    #[cfg(feature = "telemetry")]
                    {
                        current_method = registry
                            .get(&current_class)
                            .map(|c| {
                                c.methods
                                    .get(method_idx)
                                    .map(|m| m.name)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                    }
                    idx = 0;
                    continue;
                }
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                // <clinit> (static initialiser) is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }
                // Attempt to load the target class; soft-fail for unloadable.
                let loaded = registry.ensure_loaded(&callee_class, loader)?;
                let resolved = if loaded {
                    resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    )
                } else {
                    None
                };
                let (dispatch_class, callee_idx) = match resolved {
                    Some((cls, i)) => (cls, i),
                    None => {
                        // Check lambda dispatch before native fallback.
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        if stack_len > arg_count {
                            let this_pos = stack_len - arg_count - 1;
                            let actual_class_opt =
                                if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                                    heap.get(r).ok().map(|o| o.class_name)
                                } else {
                                    None
                                };

                            if let Some(ref actual_class) = actual_class_opt
                                && let Some(lambda_info) =
                                    registry.get_lambda(actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                let mut sam_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                sam_args.reverse();
                                let this_slot = frame.pop()?;
                                let this_ref = match &this_slot {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };

                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(sam_args.iter().cloned());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &lambda_info.impl_class,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        for (i, slot) in impl_args.into_iter().enumerate() {
                                            locals_buf[i] = slot;
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    call_stack.push(CallFrame {
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        resume_idx: idx + 1,
                                        class_name: current_class,
                                    });
                                    frame = callee_frame;
                                    method_idx = impl_idx;
                                    pc_to_idx = callee_pc_to_idx;
                                    current_class = dispatch_class;
                                    #[cfg(feature = "telemetry")]
                                    {
                                        current_method = registry
                                            .get(&current_class)
                                            .map(|c| {
                                                c.methods
                                                    .get(method_idx)
                                                    .map(|m| m.name)
                                                    .unwrap_or_default()
                                            })
                                            .unwrap_or_default();
                                    }
                                    idx = 0;
                                    continue;
                                }
                            }
                        }
                        // Check native registry, walking the super chain.
                        let native_handler_kind = {
                            let mut found = registry.natives.get_kind(
                                &callee_class,
                                &callee_name,
                                &callee_desc,
                            );
                            if found.is_none() {
                                // Walk super chain for native lookup (e.g. Enum.ordinal
                                // called via SimpleEnum$Color.ordinal).
                                let start = if callee_class.starts_with('[') {
                                    // Arrays inherit from Object.
                                    Some("java/lang/Object".to_string())
                                } else {
                                    registry
                                        .get(&callee_class)
                                        .ok()
                                        .and_then(|c| c.super_class)
                                };
                                let mut sc = start;
                                while let Some(ref s) = sc {
                                    if let Some(h) =
                                        registry.natives.get_kind(s, &callee_name, &callee_desc)
                                    {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class);
                                }
                            }
                            found
                        };
                        match native_handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                native_args.reverse();
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let result = handler(&native_args, heap, stdout);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        // Native methods do not perform a bytecode hierarchy
                                        // walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = result?;
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                native_args.reverse();
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut invoke_cb =
                                    |heap: &mut duke_gc::Heap,
                                     output: &mut dyn std::io::Write,
                                     class: &str,
                                     method: &str,
                                     desc: &str,
                                     cb_args: Vec<Slot>|
                                     -> VmResult<Option<Slot>> {
                                        execute_class(
                                            registry, loader, heap, output, class, method, desc,
                                            &cb_args,
                                        )
                                    };
                                let result = handler(&native_args, heap, stdout, &mut invoke_cb);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = result?;
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            None => {
                                // Unloadable or missing — pop args + this and continue.
                                let arg_count = parse_arg_count(&callee_desc);
                                for _ in 0..arg_count {
                                    frame.pop()?;
                                }
                                frame.pop()?; // pop `this`
                                idx += 1;
                                continue;
                            }
                        }
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                // Populate dispatch cache for invokespecial (static dispatch — result is stable).
                if matches!(instr, Instruction::Invokespecial(_)) {
                    dispatch_cache
                        .entry(current_class)
                        .or_default()
                        .insert(cp_idx.0, (dispatch_class, callee_idx, arg_count));
                }
                #[cfg(feature = "telemetry")]
                if matches!(instr, Instruction::Invokevirtual(_)) {
                    registry.telemetry.dispatch_resolution.record(
                        &current_class,
                        cp_idx.0,
                        &dispatch_class,
                        dispatch_class != callee_class,
                    );
                }
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop args into locals[1..=arg_count] in reverse (stack top = last arg).
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                    class_name: current_class,
                });
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                current_class = dispatch_class;
                #[cfg(feature = "telemetry")]
                {
                    current_method = registry
                        .get(&current_class)
                        .map(|c| {
                            c.methods
                                .get(method_idx)
                                .map(|m| m.name)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                }
                idx = 0;
                continue;
            }

            // ---- Reference return ----
            Instruction::Areturn => {
                let v = frame.pop()?;
                do_return!(Some(v));
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let r = heap.allocate(class_name.to_string(), count as usize);
                // Fix element types for non-int primitive arrays.
                match array_type {
                    ArrayType::Long => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Long(0);
                        }
                    }
                    ArrayType::Float => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Float(0.0);
                        }
                    }
                    ArrayType::Double => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Double(0.0);
                        }
                    }
                    _ => {} // Int/Boolean/Byte/Char/Short default to Slot::Int(0)
                }
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = heap.allocate(array_type, count as usize);
                // Fix elements to Reference(None).
                let obj = heap.get_mut(r)?;
                for slot in &mut obj.fields {
                    *slot = Slot::Reference(None);
                }
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = heap.get(r)?.fields.len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()?
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_long()?
                };
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Long(val))?;
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_float()?
                };
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Float(val))?;
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_double()?
                };
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Double(val))?;
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize]
                };
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, val)?;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()? as i8 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = frame.pop_int()? as i8 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()? as u16 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = frame.pop_int()? as u16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()? as i16 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = frame.pop_int()? as i16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(&current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name;
                        if is_assignable_from(registry, loader, &actual, &target) {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(&current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name;
                        let result =
                            i32::from(is_assignable_from(registry, loader, &actual, &target));
                        frame.push(Slot::Int(result))?;
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow with exception table dispatch ----
            Instruction::Athrow => {
                let exception_ref = frame.pop_ref()?;
                let exc_class_name = heap.get(exception_ref)?.class_name;

                #[cfg(feature = "telemetry")]
                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
                    &exc_class_name,
                    &current_class,
                    &current_method,
                    pc,
                );

                // Clone the exception table to release the borrow on registry,
                // so find_exception_handler can use &mut registry for hierarchy checks.
                let exc_table = registry.get(&current_class)?.methods[method_idx]
                    .exception_table
                    .iter()
                    .map(|e| ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type: e.catch_type,
                    })
                    .collect::<Vec<_>>();
                let handler =
                    find_exception_handler(&exc_table, pc, &exc_class_name, registry, loader);
                if let Some(handler_pc) = handler {
                    #[cfg(feature = "telemetry")]
                    registry.telemetry.exception_flow.record_catch(
                        _telem_exc_event_idx,
                        &current_class,
                        &current_method,
                        handler_pc as usize,
                    );
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        VmError::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                // No handler in current method — unwind call stack.
                loop {
                    match call_stack.pop() {
                        None => {
                            return Err(VmError::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            // Recycle callee frame buffers before overwriting `frame` —
                            // mirrors the do_return! pattern to avoid a pool leak.
                            let old = std::mem::replace(&mut frame, caller.frame);
                            let (l, s) = old.into_pool_bufs();
                            frame_pool.release(l, s);
                            method_idx = caller.method_idx;
                            pc_to_idx = caller.pc_to_idx;
                            current_class = caller.class_name;
                            #[cfg(feature = "telemetry")]
                            {
                                current_method = registry
                                    .get(&current_class)
                                    .map(|c| {
                                        c.methods
                                            .get(method_idx)
                                            .map(|m| m.name)
                                            .unwrap_or_default()
                                    })
                                    .unwrap_or_default();
                            }

                            // Clone exception table and compute caller_pc before hierarchy check.
                            let (caller_exc_table, caller_pc) = {
                                let ctx = registry.get(&current_class)?;
                                let cpc = if caller.resume_idx > 0 {
                                    ctx.methods[method_idx].instructions[caller.resume_idx - 1].0
                                } else {
                                    0
                                };
                                let tbl = ctx.methods[method_idx]
                                    .exception_table
                                    .iter()
                                    .map(|e| ExceptionEntry {
                                        start_pc: e.start_pc,
                                        end_pc: e.end_pc,
                                        handler_pc: e.handler_pc,
                                        catch_type: e.catch_type,
                                    })
                                    .collect::<Vec<_>>();
                                (tbl, cpc)
                            };
                            let handler = find_exception_handler(
                                &caller_exc_table,
                                caller_pc,
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: max operand stack depth
/// - `max_locals`: max local variable array size
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: max operand stack depth
/// - `max_locals`: max local variable array size
                                &exc_class_name,
                                registry,
                                loader,
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: max operand stack depth
/// - `max_locals`: max local variable array size
                            );

                            if let Some(handler_pc) = handler {
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.exception_flow.record_catch(
                                    _telem_exc_event_idx,
                                    &current_class,
                                    &current_method,
                                    handler_pc as usize,
                                );
                                frame.clear_stack();
                                frame.push(Slot::Reference(Some(exception_ref)))?;
                                idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    VmError::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                            // No handler here either — keep unwinding.
                        }
                    }
                }
                continue;
            }

            // ----------------------------------------------------------------
            // invokedynamic — resolve bootstrap method, dispatch based on
            // the bootstrap class (StringConcatFactory, LambdaMetafactory).
            // ----------------------------------------------------------------
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(&current_class)?;
                    let cp = &ctx.constant_pool;
                    match cp.get(cp_idx_val).and_then(|e| e.as_ref()) {
                        Some(CpEntry::InvokeDynamic {
                            bootstrap_method_attr_index,
                            name_and_type_index,
                        }) => {
                            let (name, desc) =
                                resolve_name_and_type(cp, name_and_type_index.0 as usize)?;
                            (*bootstrap_method_attr_index as usize, name, desc)
                        }
                        _ => return Err(VmError::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_class, bsm_args) = {
                    let ctx = registry.get(&current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(VmError::InvalidCpIndex { index: bsm_idx })?;
                    let (_kind, class, _name, _desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<duke_classfile::types::CpIndex> = bsm_entry.arguments;
                    (class, args)
                };

                let _ = &call_name; // suppress unused warning for now

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    let arg_count = parse_arg_count(&call_desc);
                    let arg_types = parse_arg_types(&call_desc);
                    let mut dynamic_args: Vec<Slot> = (0..arg_count)
                        .map(|_| frame.pop())
                        .collect::<VmResult<Vec<_>>>()?;
                    dynamic_args.reverse();

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if !bsm_args.is_empty() {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
                        } else {
                            String::new()
                        };
                        let mut consts = Vec::new();
                        for arg_idx in bsm_args.iter().skip(1) {
                            if let Ok(s) = resolve_cp_string(cp, arg_idx.0 as usize) {
                                consts.push(s);
                            }
                        }
                        (recipe, consts)
                    };

                    let result = execute_string_concat_recipe(
                        &recipe,
                        &dynamic_args,
                        &arg_types,
                        &constants,
                        heap,
                    )?;
                    frame.push(result)?;
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory.metafactory ---
                    // Bootstrap args: [MethodType erased, MethodHandle impl, MethodType specialized]

                    let (impl_kind, impl_class, impl_method, impl_desc) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(VmError::Unimplemented {
                                mnemonic: "LambdaMetafactory requires 3 bootstrap args",
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    let sam_method = call_name;

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s,
                                    _ => {
                                        return Err(VmError::InvalidCpIndex {
                                            index: descriptor_index.0 as usize,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(VmError::InvalidCpIndex {
                                    index: bsm_args[0].0 as usize,
                                });
                            }
                        }
                    };

                    // Pop captured variables from the stack.
                    let captured_count = parse_arg_count(&call_desc);
                    let mut captured_args: Vec<Slot> = (0..captured_count)
                        .map(|_| frame.pop())
                        .collect::<VmResult<Vec<_>>>()?;
                    captured_args.reverse();

                    let lambda_info = LambdaInfo {
                        impl_class: impl_class,
                        impl_method: impl_method,
                        impl_desc: impl_desc,
                        impl_kind,
                        sam_method,
                        sam_desc,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    let r = heap.allocate(lambda_class, captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    let _ = registry.ensure_loaded(&impl_class, loader);

                    frame.push(Slot::Reference(Some(r)))?;
                    if heap.should_gc() {
                        let roots = gather_roots(&frame, &call_stack, registry);
                        heap.collect(&roots);
                        patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                    }
                } else {
                    // Unknown bootstrap method — pop args and push null.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    if !call_desc.ends_with(")V") {
                        frame.push(Slot::Reference(None))?;
                    }
                }
            }

            // ----------------------------------------------------------------
            // invokeinterface — like invokevirtual but resolves from
            // InterfaceMethodref and dispatches on the actual object class.
            // ----------------------------------------------------------------
            Instruction::Invokeinterface {
                index: cp_idx,
                count: _,
            } => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }
                let arg_count = parse_arg_count(&callee_desc);
                // Peek at `this` (sits below the args) to determine the actual
                // runtime class without consuming the stack yet.  Each dispatch
                // path (native / lambda / bytecode) pops what it needs itself.
                let stack_len = frame.stack_len();
                let actual_class = if stack_len > arg_count {
                    let this_pos = stack_len - arg_count - 1;
                    if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                        heap.get(r).ok().map(|o| o.class_name)
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| callee_class);

                // Try to find the method on the actual class (walking hierarchy).
                let resolved = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &actual_class,
                    &callee_name,
                    &callee_desc,
                );

                let (dispatch_class, callee_idx) = if let Some(pair) = resolved {
                    pair
                } else {
                    // Fall back to interface class hierarchy.
                    let iface_resolved = resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    );
                    match iface_resolved {
                        Some(pair) => pair,
                        None => {
                            // Check native registry — try actual class then interface class.
                            let native_kind = registry
                                .natives
                                .get_kind(&actual_class, &callee_name, &callee_desc)
                                .or_else(|| {
                                    registry.natives.get_kind(
                                        &callee_class,
                                        &callee_name,
                                        &callee_desc,
                                    )
                                });
                            match native_kind {
                                Some(HandlerKind::Simple(handler)) => {
                                    // Native path: collect args + this into a Vec<Slot>
                                    // for the handler(&[Slot], ...) signature.
                                    let mut callee_args: Vec<Slot> = (0..arg_count)
                                        .map(|_| frame.pop())
                                        .collect::<VmResult<Vec<_>>>()?;
                                    callee_args.reverse();
                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let result = handler(&callee_args, heap, stdout);
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        // Native interface methods skip the bytecode
                                        // hierarchy walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = result?;
                                    if let Some(val) = result {
                                        frame.push(val)?;
                                    }
                                    idx += 1;
                                    continue;
                                }
                                Some(HandlerKind::Callback(handler)) => {
                                    let mut callee_args: Vec<Slot> = (0..arg_count)
                                        .map(|_| frame.pop())
                                        .collect::<VmResult<Vec<_>>>()?;
                                    callee_args.reverse();
                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let mut invoke_cb = |heap: &mut duke_gc::Heap,
                                                         output: &mut dyn std::io::Write,
                                                         class: &str,
                                                         method: &str,
                                                         desc: &str,
                                                         cb_args: Vec<Slot>|
                                     -> VmResult<Option<Slot>> {
                                        execute_class(
                                            registry, loader, heap, output, class, method, desc,
                                            &cb_args,
                                        )
                                    };
                                    let result =
                                        handler(&callee_args, heap, stdout, &mut invoke_cb);
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = result?;
                                    if let Some(val) = result {
                                        frame.push(val)?;
                                    }
                                    idx += 1;
                                    continue;
                                }
                                None => {} // fall through to lambda / no-op
                            }
                            // Check lambda registry for SAM dispatch.
                            if let Some(lambda_info) = registry.get_lambda(&actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // Lambda path: collect args + this into a Vec<Slot>.
                                let mut callee_args: Vec<Slot> = (0..arg_count)
                                    .map(|_| frame.pop())
                                    .collect::<VmResult<Vec<_>>>()?;
                                callee_args.reverse();
                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                let this_ref = match &callee_args[0] {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };
                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(callee_args[1..].iter().cloned());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                if lambda_info.impl_kind == 6 {
                                    // invokeStatic dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        call_stack.push(CallFrame {
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            resume_idx: idx + 1,
                                            class_name: current_class,
                                        });
                                        frame = callee_frame;
                                        method_idx = impl_idx;
                                        pc_to_idx = callee_pc_to_idx;
                                        current_class = dispatch_class;
                                        #[cfg(feature = "telemetry")]
                                        {
                                            current_method = registry
                                                .get(&current_class)
                                                .map(|c| {
                                                    c.methods
                                                        .get(method_idx)
                                                        .map(|m| m.name)
                                                        .unwrap_or_default()
                                                })
                                                .unwrap_or_default();
                                        }
                                        idx = 0;
                                        continue;
                                    }
                                } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                    // invokeVirtual / invokeInterface dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        call_stack.push(CallFrame {
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            resume_idx: idx + 1,
                                            class_name: current_class,
                                        });
                                        frame = callee_frame;
                                        method_idx = impl_idx;
                                        pc_to_idx = callee_pc_to_idx;
                                        current_class = dispatch_class;
                                        #[cfg(feature = "telemetry")]
                                        {
                                            current_method = registry
                                                .get(&current_class)
                                                .map(|c| {
                                                    c.methods
                                                        .get(method_idx)
                                                        .map(|m| m.name)
                                                        .unwrap_or_default()
                                                })
                                                .unwrap_or_default();
                                        }
                                        idx = 0;
                                        continue;
                                    }
                                    // Try native fallback for virtual/interface
                                    let lambda_native_kind = registry.natives.get_kind(
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    match lambda_native_kind {
                                        Some(HandlerKind::Simple(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let result = handler(&impl_args, heap, stdout);
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = result?;
                                            if let Some(val) = result {
                                                frame.push(val)?;
                                            }
                                            idx += 1;
                                            continue;
                                        }
                                        Some(HandlerKind::Callback(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let mut invoke_cb =
                                                |heap: &mut duke_gc::Heap,
                                                 output: &mut dyn std::io::Write,
                                                 class: &str,
                                                 method: &str,
                                                 desc: &str,
                                                 cb_args: Vec<Slot>|
                                                 -> VmResult<Option<Slot>> {
                                                    execute_class(
                                                        registry, loader, heap, output, class,
                                                        method, desc, &cb_args,
                                                    )
                                                };
                                            let result =
                                                handler(&impl_args, heap, stdout, &mut invoke_cb);
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = result?;
                                            if let Some(val) = result {
                                                frame.push(val)?;
                                            }
                                            idx += 1;
                                            continue;
                                        }
                                        None => {}
                                    }
                                }
                                idx += 1;
                                continue;
                            }
                            // No-op fallback.
                            idx += 1;
                            continue;
                        }
                    }
                };

                // Bytecode execution path — Pattern B: pop directly into locals_buf.
                #[cfg(feature = "telemetry")]
                registry.telemetry.dispatch_resolution.record(
                    &current_class,
                    cp_idx.0,
                    &dispatch_class,
                    dispatch_class != actual_class,
                );
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop method args in reverse (stack top = last arg) into locals[1..=arg_count].
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                    class_name: current_class,
                });
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                current_class = dispatch_class;
                #[cfg(feature = "telemetry")]
                {
                    current_method = registry
                        .get(&current_class)
                        .map(|c| {
                            c.methods
                                .get(method_idx)
                                .map(|m| m.name)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                }
                idx = 0;
                continue;
            }

            // ----------------------------------------------------------------
            // multianewarray — allocate multi-dimensional arrays.
            // ----------------------------------------------------------------
            Instruction::Multianewarray {
                index: cp_idx,
                dimensions,
            } => {
                let element_type = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // Pop dimension sizes from stack (first popped is rightmost dimension).
                let mut dims: Vec<i32> = Vec::new();
                for _ in 0..*dimensions {
                    dims.push(frame.pop_int()?);
                }
                dims.reverse(); // Now dims[0] is outermost.

                // Check for negative sizes.
                for &d in &dims {
                    if d < 0 {
                        return Err(VmError::NegativeArraySize { size: d });
                    }
                }

                // Recursive allocation helper.
                fn alloc_multi(
                    heap: &mut duke_gc::Heap,
                    dims: &[i32],
                    depth: usize,
                    type_name: &str,
                ) -> u64 {
                    let size = dims[depth] as usize;
                    let r = heap.allocate(type_name.to_string(), size);
                    if depth < dims.len() - 1 {
                        // Not the innermost — fill with references to sub-arrays.
                        let inner_type = &type_name[1..]; // Strip one '[' for inner dimension.
                        for i in 0..size {
                            let inner = alloc_multi(heap, dims, depth + 1, inner_type);
                            heap.get_mut(r).unwrap().fields[i] = Slot::Reference(Some(inner));
                        }
                    }
                    r
                }

                let r = alloc_multi(heap, &dims, 0, &element_type);
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter => {
                let _obj = frame.pop()?;
            }
            Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        #[cfg(feature = "telemetry")]
        {
            let elapsed = _telem_start.elapsed().as_nanos() as u64;
            registry.telemetry.bytecode_cost.record(
                _telem_name,
                &current_class,
                &current_method,
                _telem_pc,
                elapsed,
            );
        }

        idx += 1;
    }
}

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::access_flags::FieldAccessFlags;
    use duke_classfile::types::{AttributeData, CpEntry};

    // Resolve this_class -> class name string.
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf
                .constant_pool
                .get(name_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s,
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s,
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s,
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })?;
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None // catch-all (finally)
                    } else {
                        match cf
                            .constant_pool
                            .get(e.catch_type.0 as usize)
                            .and_then(|x| x.as_ref())
                        {
                            Some(CpEntry::Class { name_index }) => {
                                match cf
                                    .constant_pool
                                    .get(name_index.0 as usize)
                                    .and_then(|x| x.as_ref())
                                {
                                    Some(CpEntry::Utf8(s)) => Some(s),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    };
                    ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type,
                    }
                })
                .collect();
            let pc_to_idx_map: std::collections::HashMap<usize, usize> = instructions
                .iter()
                .enumerate()
                .map(|(i, &(pc, _))| (pc, i))
                .collect();
            Some(MethodEntry {
                name,
                descriptor,
                instructions,
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
            })
        })
        .collect();

    let mut fields = Vec::new();
    let mut static_count = 0usize;
    let mut instance_count = 0usize;

    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s,
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s,
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_count += 1;
        } else {
            instance_count += 1;
        }
        fields.push(FieldEntry {
            name,
            descriptor,
            is_static,
        });
    }

    // Resolve super_class: if the index is 0, this is java/lang/Object (no super).
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };

    // Resolve directly-implemented interfaces.
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();

    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries)
            } else {
                None
            }
        })
        .unwrap_or_default();

    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool,
        methods,
        fields,
        static_fields: vec![Slot::Int(0); static_count],
        instance_field_count: instance_count,
        bootstrap_methods,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s),
                _ => Err(VmError::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}

/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
) -> bool {
    if from == to || to == "java/lang/Object" {
        return true;
    }
    // Arrays implement Cloneable and Serializable; everything else is Object.
    if from.starts_with('[') {
        return matches!(to, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from.to_string());

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current) {
            continue;
        }
        let _ = registry.ensure_loaded(&current, loader);
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class, ctx.interfaces),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}

/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
        #[allow(clippy::option_if_let_else)] // match is clearer with &mut registry
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => is_assignable_from(registry, loader, class_name, ct),
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}

/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns (class_name_where_found, method_index) or None.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    let mut current = start_class.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current) {
            return None; // circular — bail
        }
        let _ = registry.ensure_loaded(&current, loader);
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    return Some((current, idx));
                }
                match &ctx.super_class {
                    Some(s) => current = s,
                    None => return None,
                }
            }
            Err(_) => return None,
        }
    }
}

/// Resolve a constant pool Methodref to (class_name, method_name, descriptor).
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Methodref {
            class_index,
            name_and_type_index,
        })
        | Some(CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}

/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .find(')')
        .map(|i| &descriptor[1..i])
        .unwrap_or("");
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Resolve a constant pool Fieldref to (class_name, field_name, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s,
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Resolve a MethodHandle CP entry to (reference_kind, class_name, method_name, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a NameAndType CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s,
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s,
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .find(')')
        .map(|i| &descriptor[1..i])
        .unwrap_or("");
    let mut types = Vec::new();
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                // Skip array dimensions and element type.
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}

/// Index of a named instance field within ctx.fields (non-static only).
/// Return the correct default [`Slot`] for a field with the given JVM descriptor.
///
/// Per JVMS §2.3/2.4: numeric types default to 0, reference/array types to null.
#[inline]
fn default_slot_for_descriptor(desc: &str) -> Slot {
    match desc.chars().next() {
        Some('J') => Slot::Long(0),
        Some('F') => Slot::Float(0.0),
        Some('D') => Slot::Double(0.0),
        Some('L') | Some('[') => Slot::Reference(None),
        _ => Slot::Int(0), // I, Z, B, C, S
    }
}

/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    // Collect hierarchy: class_name → … → root
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class;
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse(); // Object-first

    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                // Only write non-Int-zero defaults (avoids an unnecessary mut borrow).
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref)
                    && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}

/// Compute the absolute slot index of a named instance field within a heap
/// object whose class is `target_class` (or any subclass of it).
///
/// JVM `Fieldref` entries name the access class (often a subclass), not
/// necessarily the declaring class. This function walks the full hierarchy
/// from the root (Object) down to `target_class`, searching each class for
/// the field and accumulating the running slot offset as it goes.
///
/// Layout: root fields occupy the lowest-numbered slots; each subclass
/// appends its fields immediately after its superclass's fields.
fn field_slot_idx(registry: &ClassRegistry, target_class: &str, name: &str) -> VmResult<usize> {
    // Build chain from target_class up to the root, then reverse for Object-first.
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(target_class.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class;
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse();

    let mut slot = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            let instance_fields: Vec<_> = ctx.fields.iter().filter(|f| !f.is_static).collect();
            if let Some(local_idx) = instance_fields.iter().position(|f| f.name == name) {
                return Ok(slot + local_idx);
            }
            slot += instance_fields.len();
        }
    }

    Err(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within ctx.static_fields.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().cloned());
    }
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    for ctx in registry.all_classes_mut() {
        for slot in ctx.static_fields.iter_mut() {
            heap.apply_forward(slot);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Unit tests: hand-crafted instruction streams ----

    #[test]
    fn class_registry_all_classes_iterates_registered() {
        let reg = ClassRegistry::new();
        // registry starts empty; verify iteration works
        let count = reg.all_classes().count();
        assert_eq!(count, 0); // before bootstrap
    }

    #[test]
    fn native_registry_register_callback_can_be_looked_up() {
        fn dummy_cb(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
            _invoke: &mut InvokeFn<'_>,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }
        let mut reg = NativeRegistry::new();
        reg.register_callback("Test", "method", "()V", dummy_cb);
        assert!(matches!(
            reg.get_kind("Test", "method", "()V"),
            Some(HandlerKind::Callback(_))
        ));
    }

    #[test]
    fn native_registry_register_simple_stays_simple() {
        fn dummy(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }
        let mut reg = NativeRegistry::new();
        reg.register("Test", "method", "()V", dummy);
        assert!(matches!(
            reg.get_kind("Test", "method", "()V"),
            Some(HandlerKind::Simple(_))
        ));
    }

    #[test]
    fn execute_iconst_ireturn() {
        let instructions = vec![(0, Instruction::Iconst1), (1, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).expect("should execute");
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn execute_bipush_istore_iload() {
        // bipush 42, istore_0, iload_0, ireturn
        let instructions = vec![
            (0, Instruction::Bipush(42)),
            (2, Instruction::Istore0),
            (3, Instruction::Iload0),
            (4, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn execute_loads_args() {
        // iload_0, iload_1, pop, ireturn — returns first arg
        let instructions = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Pop),
            (3, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![Slot::Int(99), Slot::Int(0)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn execute_sipush() {
        let instructions = vec![(0, Instruction::Sipush(1000)), (3, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1000)));
    }

    #[test]
    fn hand_coded_add() {
        // iload_0, iload_1, iadd, ireturn
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Iadd),
            (3, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![Slot::Int(3), Slot::Int(4)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn hand_coded_div_by_zero() {
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Idiv),
            (3, Instruction::Ireturn),
        ];
        let err = execute(&instrs, &[], vec![Slot::Int(10), Slot::Int(0)], 2, 2).unwrap_err();
        assert!(matches!(err, VmError::DivisionByZero));
    }

    #[test]
    fn hand_coded_long_add() {
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Ladd),
            (3, Instruction::Lreturn),
        ];
        let result = execute(
            &instrs,
            &[],
            vec![Slot::Long(1_000_000_000), Slot::Long(2_000_000_000)],
            2,
            2,
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(3_000_000_000)));
    }

    #[test]
    fn hand_coded_ifeq_taken() {
        // iconst_0 (pc=0), ifeq +5 (pc=1, target=6), iconst_1 (pc=4), ireturn (pc=5),
        // iconst_2 (pc=6), ireturn (pc=7)
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)), // 0 == 0, jump to pc=6
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn hand_coded_ifeq_not_taken() {
        let instrs = vec![
            (0, Instruction::Iconst1), // push 1
            (1, Instruction::Ifeq(5)), // 1 != 0, NOT taken
            (4, Instruction::Iconst1), // reached
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hand_coded_goto() {
        // iconst_5 (pc=0), goto +4 (pc=1, target=5), pop (pc=4, skipped),
        // ireturn (pc=5) — returns 5
        let instrs = vec![
            (0, Instruction::Iconst5),
            (1, Instruction::Goto(4)), // jump to pc=5
            (4, Instruction::Pop),     // skipped
            (5, Instruction::Ireturn), // returns 5
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    // ---- Integration tests: load Arithmetic.class and execute real bytecode ----

    fn fixture(name: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures");
        p.push(name);
        p
    }

    /// Parse and execute a static method from a `.class` file that takes `i32` args
    /// and returns an `i32`.
    fn run_static_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
        use duke_bytecode::decode;
        use duke_classfile::{
            parse,
            types::{AttributeData, CpEntry},
        };

        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");

        let method = cf
            .methods
            .iter()
            .find(|m| {
                let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
                else {
                    return false;
                };
                s.as_str() == method_name
            })
            .unwrap_or_else(|| panic!("method '{method_name}' not found"));

        let code = method
            .attributes
            .iter()
            .find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("no Code attribute");

        let instructions = decode(&code.code).expect("decode failed");
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();

        match execute(
            &instructions,
            &cf.constant_pool,
            slots,
            code.max_stack,
            code.max_locals,
        )
        .expect("execute failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    // Basic arithmetic
    #[test]
    fn int_add() {
        assert_eq!(run_static_int("Arithmetic.class", "add", vec![3, 4]), 7);
    }
    #[test]
    fn int_subtract() {
        assert_eq!(
            run_static_int("Arithmetic.class", "subtract", vec![10, 3]),
            7
        );
    }
    #[test]
    fn int_multiply() {
        assert_eq!(
            run_static_int("Arithmetic.class", "multiply", vec![3, 4]),
            12
        );
    }
    #[test]
    fn int_divide() {
        assert_eq!(run_static_int("Arithmetic.class", "divide", vec![10, 2]), 5);
    }
    #[test]
    fn int_remainder() {
        assert_eq!(
            run_static_int("Arithmetic.class", "remainder", vec![10, 3]),
            1
        );
    }
    #[test]
    fn int_negate() {
        assert_eq!(run_static_int("Arithmetic.class", "negate", vec![-5]), 5);
    }
    #[test]
    fn int_shift_left() {
        assert_eq!(
            run_static_int("Arithmetic.class", "shiftLeft", vec![1, 4]),
            16
        );
    }
    #[test]
    fn int_bitwise_and() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseAnd", vec![0b1111, 0b1010]),
            0b1010
        );
    }
    #[test]
    fn int_bitwise_or() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseOr", vec![0b1111, 0b1010]),
            0b1111
        );
    }

    // Conditionals
    #[test]
    fn int_max_a_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![7, 3]), 7);
    }
    #[test]
    fn int_max_b_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![3, 7]), 7);
    }
    #[test]
    fn int_abs_neg() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![-5]), 5);
    }
    #[test]
    fn int_abs_pos() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![5]), 5);
    }
    #[test]
    fn int_clamp_mid() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![5, 1, 10]),
            5
        );
    }
    #[test]
    fn int_clamp_lo() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![0, 1, 10]),
            1
        );
    }
    #[test]
    fn int_clamp_hi() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![15, 1, 10]),
            10
        );
    }

    // Control flow / loops
    #[test]
    fn int_factorial_0() {
        assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![0]), 1);
    }
    #[test]
    fn int_factorial_5() {
        assert_eq!(
            run_static_int("Arithmetic.class", "factorial", vec![5]),
            120
        );
    }
    #[test]
    fn int_factorial_10() {
        assert_eq!(
            run_static_int("Arithmetic.class", "factorial", vec![10]),
            3_628_800
        );
    }
    #[test]
    fn int_fibonacci_0() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![0]), 0);
    }
    #[test]
    fn int_fibonacci_1() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![1]), 1);
    }
    #[test]
    fn int_fibonacci_10() {
        assert_eq!(
            run_static_int("Arithmetic.class", "fibonacci", vec![10]),
            55
        );
    }
    #[test]
    fn int_sum_to_100() {
        assert_eq!(run_static_int("Arithmetic.class", "sumTo", vec![100]), 5050);
    }

    // ---- Phase 5: ClassContext + execute_class() tests ----

    fn load_class_context(class_name: &str) -> ClassContext {
        use duke_classfile::parse;
        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");
        build_class_context(&cf)
    }

    fn run_class_int(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i32 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn class_square() {
        assert_eq!(
            run_class_int("MathUtils.class", "square", "(I)I", vec![7]),
            49
        );
    }

    #[test]
    fn class_sum_of_squares_3_4() {
        assert_eq!(
            run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![3, 4]),
            25
        );
    }

    #[test]
    fn class_sum_of_squares_5_12() {
        assert_eq!(
            run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![5, 12]),
            169
        );
    }

    #[test]
    fn class_power_2_10() {
        assert_eq!(
            run_class_int("MathUtils.class", "power", "(II)I", vec![2, 10]),
            1024
        );
    }

    #[test]
    fn class_gcd_48_18() {
        assert_eq!(
            run_class_int("MathUtils.class", "gcd", "(II)I", vec![48, 18]),
            6
        );
    }

    #[test]
    fn class_method_not_found() {
        let ctx = load_class_context("MathUtils.class");
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let err = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            "nonExistent",
            "(I)I",
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, VmError::MethodNotFound { .. }));
    }

    // ---- Unit tests: parse_arg_count ----

    #[test]
    fn arg_count_empty() {
        assert_eq!(parse_arg_count("()V"), 0);
    }

    #[test]
    fn arg_count_single_int() {
        assert_eq!(parse_arg_count("(I)I"), 1);
    }

    #[test]
    fn arg_count_two_ints() {
        assert_eq!(parse_arg_count("(II)I"), 2);
    }

    #[test]
    fn arg_count_long_double() {
        assert_eq!(parse_arg_count("(JD)V"), 2);
    }

    #[test]
    fn arg_count_object_ref() {
        assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
    }

    #[test]
    fn arg_count_array() {
        assert_eq!(parse_arg_count("([II)I"), 2);
    }

    #[test]
    fn arg_count_mixed() {
        assert_eq!(parse_arg_count("(ILjava/lang/Object;Z)V"), 3);
    }

    // ---- Unit tests: resolve_methodref ----

    fn make_cp(entries: Vec<Option<CpEntry>>) -> Vec<Option<CpEntry>> {
        let mut cp = vec![None]; // slot 0 reserved
        cp.extend(entries);
        cp
    }

    #[test]
    fn resolve_methodref_valid() {
        use duke_classfile::types::CpIndex;
        let cp = make_cp(vec![
            Some(CpEntry::Methodref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class {
                name_index: CpIndex(6),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("square".to_string())),
            Some(CpEntry::Utf8("(I)I".to_string())),
            Some(CpEntry::Utf8("MathUtils".to_string())),
        ]);
        let (class_name, name, desc) = resolve_methodref(&cp, 1).unwrap();
        assert_eq!(class_name, "MathUtils");
        assert_eq!(name, "square");
        assert_eq!(desc, "(I)I");
    }

    #[test]
    fn resolve_methodref_invalid_index() {
        let cp = make_cp(vec![]);
        let err = resolve_methodref(&cp, 99).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { index: 99 }));
    }

    #[test]
    fn resolve_methodref_not_a_methodref() {
        let cp = make_cp(vec![Some(CpEntry::Utf8("not a methodref".to_string()))]);
        let err = resolve_methodref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { .. }));
    }

    // ---- Phase 6: object creation + field access ----

    #[test]
    fn point_sum_1_2_3_4() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]),
            10
        );
    }

    #[test]
    fn point_sum_3_4_0_0() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 0, 0]),
            7
        );
    }

    #[test]
    fn point_sum_zeros() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![0, 0, 0, 0]),
            0
        );
    }

    #[test]
    fn point_sum_symmetry() {
        let a = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]);
        let b = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 1, 2]);
        assert_eq!(a, b);
    }

    // ---- Unit tests: resolve_fieldref ----

    #[test]
    fn resolve_fieldref_valid() {
        use duke_classfile::types::CpIndex;
        let cp = make_cp(vec![
            Some(CpEntry::Fieldref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class {
                name_index: CpIndex(6),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("x".to_string())),
            Some(CpEntry::Utf8("I".to_string())),
            Some(CpEntry::Utf8("Point".to_string())),
        ]);
        let (class_name, name, desc) = resolve_fieldref(&cp, 1).unwrap();
        assert_eq!(class_name, "Point");
        assert_eq!(name, "x");
        assert_eq!(desc, "I");
    }

    #[test]
    fn resolve_fieldref_invalid() {
        let cp = make_cp(vec![Some(CpEntry::Utf8("not a fieldref".to_string()))]);
        let err = resolve_fieldref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidFieldref { .. }));
    }

    // ---- Phase 7: Arrays ----

    fn run_class_long(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> i64 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Long(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    fn run_class_double(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> f64 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Double(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn array_sum_5() {
        // sumArray(5) = 1+2+3+4+5 = 15
        assert_eq!(
            run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![5]),
            15
        );
    }

    #[test]
    fn array_length() {
        assert_eq!(
            run_class_int("ArrayOps.class", "arrayLength", "(I)I", vec![7]),
            7
        );
    }

    #[test]
    fn array_sum_long() {
        // sumLongArray(3) = 0*1e6 + 1*1e6 + 2*1e6 = 3_000_000
        assert_eq!(
            run_class_long("ArrayOps.class", "sumLongArray", "(I)J", vec![3]),
            3_000_000
        );
    }

    #[test]
    fn array_first_double() {
        // firstDouble(3): a[0] = 0 * 0.5 = 0.0
        let result = run_class_double("ArrayOps.class", "firstDouble", "(I)D", vec![3]);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn array_empty_sum() {
        // sumArray(0) = sum of empty = 0
        assert_eq!(
            run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![0]),
            0
        );
    }

    // ---- Phase 7: Array unit tests (no fixture needed) ----

    #[test]
    fn newarray_int_arraylength() {
        // newarray T_INT count=3 -> arraylength -> 3
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst3),
            (1, Newarray(ArrayType::Int)),
            (3, Arraylength),
            (4, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn newarray_iastore_iaload() {
        use duke_bytecode::Instruction::*;
        // int[] a = new int[1]; a[0] = 42; return a[0];
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Dup),
            (4, Iconst0),
            (5, Bipush(42)),
            (7, Iastore),
            (8, Iconst0),
            (9, Iaload),
            (10, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn newarray_int_bounds_error() {
        use duke_bytecode::Instruction::*;
        // new int[1], then iaload at index 5 -> ArrayIndexOutOfBounds
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Bipush(5)),
            (5, Iaload),
            (6, Ireturn),
        ];
        let err = execute(&instrs, &[], vec![], 3, 0).unwrap_err();
        assert!(matches!(
            err,
            VmError::ArrayIndexOutOfBounds {
                index: 5,
                length: 1
            }
        ));
    }

    #[test]
    fn newarray_negative_size() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, IconstM1),
            (1, Newarray(ArrayType::Int)),
            (3, Arraylength),
            (4, Ireturn),
        ];
        let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
    }

    #[test]
    fn athrow_propagates_as_java_exception() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![(0, Iconst1), (1, Newarray(ArrayType::Int)), (3, Athrow)];
        let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));
    }

    // ---- Phase 8: Exceptions ----

    #[test]
    fn exception_catch_simple() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "catchSimple", "()I", vec![]),
            42
        );
    }

    #[test]
    fn exception_uncaught_propagates() {
        let ctx = load_class_context("ExceptionTest.class");
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            "uncaught",
            "()I",
            &[],
        );
        let err = result.unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));
    }

    #[test]
    fn exception_finally_block() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "finallyBlock", "()I", vec![]),
            11 // 10 + 1
        );
    }

    #[test]
    fn exception_catch_from_callee() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "catchFromCallee", "()I", vec![]),
            99
        );
    }

    // ---- Phase 8: Switch statements ----

    #[test]
    fn switch_dense_case0() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![0]),
            10
        );
    }

    #[test]
    fn switch_dense_case2() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![2]),
            30
        );
    }

    #[test]
    fn switch_dense_default() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![99]),
            -1
        );
    }

    #[test]
    fn switch_sparse_case200() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![200]),
            2
        );
    }

    #[test]
    fn switch_sparse_default() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![999]),
            0
        );
    }

    // ---- Phase 8: Switch (unit tests) ----

    #[test]
    fn tableswitch_match() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (
                1,
                Tableswitch {
                    default: 100,
                    low: 0,
                    high: 2,
                    offsets: vec![10, 20, 30],
                },
            ),
            (11, Bipush(10)),
            (13, Ireturn),
            (21, Bipush(20)),
            (23, Ireturn),
            (31, Bipush(30)),
            (33, Ireturn),
            (101, Bipush(-1)),
            (103, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(20))); // case 1
    }

    #[test]
    fn tableswitch_default() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Bipush(99)),
            (
                2,
                Tableswitch {
                    default: 100,
                    low: 0,
                    high: 2,
                    offsets: vec![10, 20, 30],
                },
            ),
            (12, Bipush(10)),
            (14, Ireturn),
            (22, Bipush(20)),
            (24, Ireturn),
            (32, Bipush(30)),
            (34, Ireturn),
            (102, Bipush(-1)),
            (104, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn lookupswitch_match() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Sipush(200)),
            (
                3,
                Lookupswitch {
                    default: 100,
                    pairs: vec![(100, 10), (200, 20), (300, 30)],
                },
            ),
            (13, Bipush(1)),
            (15, Ireturn),
            (23, Bipush(2)),
            (25, Ireturn),
            (33, Bipush(3)),
            (35, Ireturn),
            (103, Bipush(0)),
            (105, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn lookupswitch_default() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Sipush(999)),
            (
                3,
                Lookupswitch {
                    default: 100,
                    pairs: vec![(100, 10), (200, 20), (300, 30)],
                },
            ),
            (13, Bipush(1)),
            (15, Ireturn),
            (23, Bipush(2)),
            (25, Ireturn),
            (33, Bipush(3)),
            (35, Ireturn),
            (103, Bipush(0)),
            (105, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    // ---- Phase 9: String constants ----

    #[test]
    fn string_non_null() {
        let result = run_class_int("StringAndTypes.class", "stringNonNull", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn string_intern() {
        let result = run_class_int("StringAndTypes.class", "stringIntern", "()I", vec![]);
        assert_eq!(result, 1);
    }

    // ---- Phase 9: instanceof ----

    #[test]
    fn instanceof_match() {
        let result = run_class_int("StringAndTypes.class", "instanceOfMatch", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn instanceof_mismatch() {
        let result = run_class_int("StringAndTypes.class", "instanceOfMismatch", "()I", vec![]);
        assert_eq!(result, 0);
    }

    #[test]
    fn instanceof_null() {
        let result = run_class_int("StringAndTypes.class", "instanceOfNull", "()I", vec![]);
        assert_eq!(result, 0);
    }

    // ---- Phase 9: checkcast ----

    #[test]
    fn checkcast_ok() {
        let result = run_class_int("StringAndTypes.class", "checkcastOk", "()I", vec![]);
        assert_eq!(result, 42);
    }

    // ---- Phase 9: Reference comparison ----

    #[test]
    fn ref_equal() {
        let result = run_class_int("StringAndTypes.class", "refEqual", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn ref_not_equal() {
        let result = run_class_int("StringAndTypes.class", "refNotEqual", "()I", vec![]);
        assert_eq!(result, 1);
    }

    // ---- Phase 9: if_acmpeq / if_acmpne unit tests ----

    #[test]
    fn if_acmpeq_same_ref() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Dup),
            (4, IfAcmpeq(10)),
            (7, Iconst0),
            (8, Ireturn),
            (14, Iconst1),
            (15, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn if_acmpne_different_refs() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Iconst1),
            (4, Newarray(ArrayType::Int)),
            (6, IfAcmpne(10)),
            (9, Iconst0),
            (10, Ireturn),
            (16, Iconst1),
            (17, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 10: Cross-class dispatch ----

    fn run_cross_class_int(
        class_files: &[&str],
        entry_class: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> i32 {
        let mut registry = ClassRegistry::new();
        for name in class_files {
            let ctx = load_class_context(name);
            registry.register(ctx);
        }
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn cross_class_add() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callAdd",
                "(II)I",
                vec![3, 4],
            ),
            7
        );
    }

    #[test]
    fn cross_class_double() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callDouble",
                "(I)I",
                vec![5],
            ),
            10
        );
    }

    #[test]
    fn cross_class_chain() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "chainCall",
                "(I)I",
                vec![3],
            ),
            9
        );
    }

    #[test]
    fn cross_class_add_negated() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "addNegated",
                "(I)I",
                vec![5],
            ),
            0
        );
    }

    #[test]
    fn cross_class_pair_sum() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairSum",
                "(II)I",
                vec![3, 7],
            ),
            10
        );
    }

    #[test]
    fn cross_class_pair_diff() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairDiff",
                "(II)I",
                vec![10, 3],
            ),
            7
        );
    }

    #[test]
    fn cross_class_two_pairs() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "twoPairsSum",
                "(IIII)I",
                vec![1, 2, 3, 4],
            ),
            10
        );
    }

    #[test]
    fn native_registry_stores_and_retrieves() {
        fn dummy_handler(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
        ) -> VmResult<Option<Slot>> {
            Ok(Some(Slot::Int(99)))
        }
        let mut natives = NativeRegistry::new();
        natives.register("Foo", "bar", "(I)I", dummy_handler);
        let handler = natives.get("Foo", "bar", "(I)I");
        assert!(handler.is_some());
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let result = handler.unwrap()(&[Slot::Int(1)], &mut heap, &mut out).unwrap();
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn native_registry_returns_none_for_missing() {
        let natives = NativeRegistry::new();
        assert!(natives.get("Foo", "bar", "(I)I").is_none());
    }

    // ---- Phase 11: native println tests ----

    fn load_hello_class() -> ClassContext {
        let bytes = std::fs::read(fixture("Hello.class")).expect("Hello.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn hello_greet_prints_to_output() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "greet",
            "()V",
            &[],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "Hello, Duke!\n");
    }

    #[test]
    fn hello_print_num() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "printNum",
            "(I)V",
            &[Slot::Int(42)],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "42\n");
    }

    #[test]
    fn hello_greet_and_return() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "greetAndReturn",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
        assert_eq!(String::from_utf8_lossy(&out), "Greetings!\n");
    }

    #[test]
    fn hello_blank_line() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "blankLine",
            "()V",
            &[],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "\n");
    }

    // ---- Phase 12: stack manipulation tests ----

    #[test]
    fn stack_ops_dup_x1() {
        // dup_x1: ..., v2, v1 → ..., v1, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst2), // push 2 (v2)
            (1, Instruction::Iconst3), // push 3 (v1)
            (2, Instruction::DupX1),   // → 3, 2, 3
            (3, Instruction::Iadd),    // → 3, 5
            (4, Instruction::Iadd),    // → 8
            (5, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(8)));
    }

    #[test]
    fn stack_ops_dup_x2() {
        // dup_x2: ..., v3, v2, v1 → ..., v1, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // push 1 (v3)
            (1, Instruction::Iconst2), // push 2 (v2)
            (2, Instruction::Iconst3), // push 3 (v1)
            (3, Instruction::DupX2),   // → 3, 1, 2, 3
            (4, Instruction::Iadd),    // → 3, 1, 5
            (5, Instruction::Iadd),    // → 3, 6
            (6, Instruction::Iadd),    // → 9
            (7, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(9)));
    }

    #[test]
    fn stack_ops_dup2() {
        // dup2: ..., v2, v1 → ..., v2, v1, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst4), // push 4 (v2)
            (1, Instruction::Iconst5), // push 5 (v1)
            (2, Instruction::Dup2),    // → 4, 5, 4, 5
            (3, Instruction::Iadd),    // → 4, 5, 9
            (4, Instruction::Iadd),    // → 4, 14
            (5, Instruction::Iadd),    // → 18
            (6, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(18)));
    }

    #[test]
    fn stack_ops_dup2_x1() {
        // dup2_x1: ..., v3, v2, v1 → ..., v2, v1, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // 1 (v3)
            (1, Instruction::Iconst2), // 2 (v2)
            (2, Instruction::Iconst3), // 3 (v1)
            (3, Instruction::Dup2X1),  // → 2, 3, 1, 2, 3
            (4, Instruction::Iadd),    // → 2, 3, 1, 5
            (5, Instruction::Iadd),    // → 2, 3, 6
            (6, Instruction::Iadd),    // → 2, 9
            (7, Instruction::Iadd),    // → 11
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(11)));
    }

    #[test]
    fn stack_ops_dup2_x2() {
        // dup2_x2: ..., v4, v3, v2, v1 → ..., v2, v1, v4, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // 1 (v4)
            (1, Instruction::Iconst2), // 2 (v3)
            (2, Instruction::Iconst3), // 3 (v2)
            (3, Instruction::Iconst4), // 4 (v1)
            (4, Instruction::Dup2X2),  // → 3, 4, 1, 2, 3, 4
            (5, Instruction::Iadd),    // → 3, 4, 1, 2, 7
            (6, Instruction::Iadd),    // → 3, 4, 1, 9
            (7, Instruction::Iadd),    // → 3, 4, 10
            (8, Instruction::Iadd),    // → 3, 14
            (9, Instruction::Iadd),    // → 17
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(17)));
    }

    // ---- Phase 12: static initializer tests ----

    fn load_static_init_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StaticInit.class")).expect("StaticInit.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn fixtures_loader() -> duke_loader::DirectoryLoader {
        duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        )
    }

    #[test]
    fn clinit_initializes_static_field_x() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getX",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn clinit_initializes_dependent_field_y() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getY",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(50)));
    }

    #[test]
    fn clinit_runs_static_block() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getZ",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(92)));
    }

    #[test]
    fn clinit_sum_all_statics() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "sum",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(184)));
    }

    // ---- Phase 12: stack ops integration tests ----

    fn load_stack_ops_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StackOps.class")).expect("StackOps.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn stack_ops_array_store_dup() {
        let ctx = load_stack_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StackOps",
            "arrayStoreDup",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(60)));
    }

    #[test]
    fn stack_ops_multi_assign() {
        let ctx = load_stack_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StackOps",
            "multiAssign",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    // ---- Phase 13: class hierarchy tests ----

    fn load_hierarchy_class() -> ClassContext {
        let bytes = std::fs::read(fixture("Hierarchy.class")).expect("Hierarchy.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn load_exception_hierarchy_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("ExceptionHierarchy.class")).expect("ExceptionHierarchy.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn load_string_ops_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringOps.class")).expect("StringOps.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn hierarchy_instanceof_object() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "instanceOfObject",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hierarchy_cast_to_object() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "castToObject",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hierarchy_null_instanceof() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "nullInstanceOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    // ---- Phase 13: exception hierarchy tests ----

    #[test]
    fn exception_hierarchy_catch_parent() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchParent",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn exception_hierarchy_catch_exact() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchExact",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn exception_hierarchy_catch_wrong_then_right() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchWrongThenRight",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn exception_hierarchy_catch_grandparent() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchGrandparent",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(4)));
    }

    // ---- Phase 13: string ops tests ----

    #[test]
    fn string_ops_length() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringLength",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_ops_equals() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringEquals",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops_not_equals() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringNotEquals",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    #[test]
    fn string_ops_char_at() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "charAtOne",
            "()I",
            &[],
        )
        .unwrap();
        // 'e' = 101
        assert_eq!(result, Some(Slot::Int(101)));
    }

    // ---- Phase 14: interface tests ----

    fn load_class(name: &str) -> ClassContext {
        let bytes = std::fs::read(fixture(name)).unwrap_or_else(|_| panic!("{name}"));
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn interface_call_simple() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "callSimple",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn interface_call_double() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "callDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(14)));
    }

    #[test]
    fn interface_polymorphic_simple() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "polymorphic",
            "(I)I",
            &[Slot::Int(0)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(8)));
    }

    #[test]
    fn interface_polymorphic_double() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "polymorphic",
            "(I)I",
            &[Slot::Int(1)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(16)));
    }

    // ---- Phase 14: multi-dimensional array tests ----

    #[test]
    fn multiarray_sum2d() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MultiArray.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MultiArray",
            "sum2d",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(21)));
    }

    #[test]
    fn multiarray_dimensions() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MultiArray.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MultiArray",
            "dimensions",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(12)));
    }

    // ---- Phase 14: native method tests ----

    #[test]
    fn more_natives_object_hashcode() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MoreNatives",
            "objectHashCode",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn more_natives_value_of_int() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MoreNatives",
            "valueOfInt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn more_natives_print_no_newline() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MoreNatives",
            "printNoNewline",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert_eq!(String::from_utf8_lossy(&out), "ABCD\n");
    }

    // ---- Phase 15: inherited method tests ----

    fn load_inherited_method_classes(registry: &mut ClassRegistry) {
        registry.register(load_class("InheritedMethod.class"));
        registry.register(load_class("InheritedMethod$Animal.class"));
        registry.register(load_class("InheritedMethod$Dog.class"));
        registry.register(load_class("InheritedMethod$Puppy.class"));
    }

    #[test]
    fn inherited_call_inherited() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callInherited",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn inherited_call_overridden() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callOverridden",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn inherited_call_deep_inherited() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callDeepInherited",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn inherited_call_deep_overridden() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callDeepOverridden",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    // ---- Phase 15: string methods tests ----

    #[test]
    fn string_methods_substring() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testSubstring",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_substring_range() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testSubstringRange",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_indexof() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIndexOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_indexof_not_found() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIndexOfNotFound",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn string_methods_contains() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testContains",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_isempty() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIsEmpty",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_compareto() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testCompareTo",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_startswith() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testStartsWith",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_endswith() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testEndsWith",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_trim() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testTrim",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn string_methods_tochararray() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testToCharArray",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(131)));
    }

    // ---- Phase 15: main entry point tests ----

    #[test]
    fn main_hello_no_args() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MainHello.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Build empty String[] array on the heap.
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
        let main_args = vec![Slot::Reference(Some(arr_ref))];

        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MainHello",
            "main",
            "([Ljava/lang/String;)V",
            &main_args,
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "no args\n");
    }

    #[test]
    fn main_hello_with_args() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MainHello.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Build String[] with ["Alice", "Bob"] on the heap.
        let alice_ref = heap.allocate_string("Alice".to_string());
        let bob_ref = heap.allocate_string("Bob".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(alice_ref));
        heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(bob_ref));
        let main_args = vec![Slot::Reference(Some(arr_ref))];

        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MainHello",
            "main",
            "([Ljava/lang/String;)V",
            &main_args,
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "Alice\nBob\n");
    }

    // ---- Phase 16: PrintAll integration tests ----

    fn load_print_all_class() -> ClassContext {
        let bytes = std::fs::read(fixture("PrintAll.class")).expect("PrintAll.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn print_all_long() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printLong",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("9876543210"),
            "stdout should contain 9876543210, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_double() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("3.14"),
            "stdout should contain 3.14, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_float() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printFloat",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("2.5"),
            "stdout should contain 2.5, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_boolean() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        let output = String::from_utf8_lossy(&out);
        assert!(
            output.contains("true") && output.contains("false"),
            "stdout should contain true and false, got: {output}",
        );
    }

    #[test]
    fn print_all_char() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printChar",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains('Z'),
            "stdout should contain Z, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_mixed() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printMixed",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert_eq!(String::from_utf8_lossy(&out), "val=42\n");
    }

    // ---- Phase 16: ParseArgs integration tests ----

    fn load_parse_args_class() -> ClassContext {
        let bytes = std::fs::read(fixture("ParseArgs.class")).expect("ParseArgs.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn parse_args_parse_int() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Build String[] with ["123"]
        let s_ref = heap.allocate_string("123".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "parseInt",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(123)));
    }

    #[test]
    fn parse_args_add_parsed() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Build String[] with ["10", "20"]
        let s1 = heap.allocate_string("10".to_string());
        let s2 = heap.allocate_string("20".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s1));
        heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(s2));
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "addParsed",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(30)));
    }

    #[test]
    fn parse_args_valueof() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "valueOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn parse_args_math_max() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathMax",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn parse_args_math_min() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathMin",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn parse_args_math_abs() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathAbs",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    // ---- Phase 16: StringConcat integration tests ----

    fn load_string_concat_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringConcat.class")).expect("StringConcat.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_concat_length() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "concatLength",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn string_concat_bool_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "boolToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(4)));
    }

    #[test]
    fn string_concat_long_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "longToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn string_concat_char_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "charToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_double_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "doubleToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 17: StringConcatFactory (invokedynamic) ----

    fn load_string_concat_test_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("StringConcatTest.class")).expect("StringConcatTest.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_concat_simple() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testSimple",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_int() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testInt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_chain() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testChain",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_boolean() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_empty() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testEmpty",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 17: LambdaMetafactory ----

    fn load_lambda_test_class() -> ClassContext {
        let bytes = std::fs::read(fixture("LambdaTest.class")).expect("LambdaTest.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn lambda_simple_no_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn lambda_with_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testCapture",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(107)));
    }

    #[test]
    fn lambda_method_reference() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testMethodRef",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-42)));
    }

    #[test]
    fn lambda_multi_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testMultiCapture",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(33)));
    }

    // ---- MonitorAndAbstract fixture ----

    fn load_monitor_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("MonitorAndAbstract.class")).expect("MonitorAndAbstract.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn monitor_sync_block() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "syncBlock",
            "(I)I",
            &[Slot::Int(7)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(14)));
    }

    #[test]
    fn monitor_sync_method() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "syncMethod",
            "(I)I",
            &[Slot::Int(5)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    #[test]
    fn monitor_nested_sync() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "nestedSync",
            "(I)I",
            &[Slot::Int(10)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    // ---- ExtendedMath integration tests ----

    fn load_extended_math_class() -> ClassContext {
        let bytes = std::fs::read(fixture("ExtendedMath.class")).expect("ExtendedMath.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn extended_math_sqrt() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testSqrt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_pow() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testPow",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_floor_ceil() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testFloorCeil",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_round() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testRound",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(4)));
    }

    #[test]
    fn extended_math_abs_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testAbsLong",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_abs_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testAbsDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_max_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMaxLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(200)));
    }

    #[test]
    fn extended_math_min_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMinLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(100)));
    }

    #[test]
    fn extended_math_max_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMaxDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(9_876_543_210)));
    }

    #[test]
    fn extended_math_parse_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_float() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseFloat",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_boolean() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_long_valueof() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testLongValueOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_constants() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMathConstants",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- StringOps2 integration tests ----

    fn load_string_ops2_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringOps2.class")).expect("StringOps2.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_ops2_to_upper_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToUpperCase",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_to_lower_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToLowerCase",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_char() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplace",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_string() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplaceString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_split() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testSplit",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_hashcode() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testHashCode",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_tostring() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToStringIdentity",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_charsequence() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplaceCharSequence",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn class_literal_non_null() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testStringClass", "()I", vec![]),
            1
        );
    }

    #[test]
    fn class_literal_self_ref() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testPrimitiveClass", "()I", vec![]),
            1
        );
    }

    #[test]
    fn class_literal_interning() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testClassInterning", "()I", vec![]),
            1
        );
    }

    // ---- Phase 19: Enum integration tests ----

    /// Helper that loads a class, calls bootstrap_stdlib, and runs a static method.
    fn run_bootstrap_int(class_name: &str, method_name: &str, descriptor: &str) -> i32 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &[],
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[cfg(feature = "telemetry")]
    fn run_fixture(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> (Option<Slot>, ClassRegistry) {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &[],
        )
        .expect("execute_class failed");
        (result, registry)
    }

    #[test]
    fn enum_ordinal() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testOrdinal", "()I"),
            1
        );
    }

    #[test]
    fn enum_name_length() {
        assert_eq!(run_bootstrap_int("SimpleEnum.class", "testName", "()I"), 3);
    }

    #[test]
    fn enum_values_length() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testValues", "()I"),
            3
        );
    }

    #[test]
    fn enum_valueof() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testValueOf", "()I"),
            2
        );
    }

    #[test]
    fn enum_switch() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testSwitch", "()I"),
            20
        );
    }

    #[test]
    fn enum_equality() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testEquality", "()I"),
            1
        );
    }

    // ---- StringBuilder tests ----

    #[test]
    fn sb_basic_append() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testBasicAppend", "()I"),
            5
        );
    }

    #[test]
    fn sb_chaining() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testChaining", "()I"),
            6
        );
    }

    #[test]
    fn sb_append_int() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendInt", "()I"),
            6
        );
    }

    #[test]
    fn sb_append_long() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendLong", "()I"),
            3
        );
    }

    #[test]
    fn sb_append_boolean() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendBoolean", "()I"),
            4
        );
    }

    #[test]
    fn sb_append_char() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendChar", "()I"),
            1
        );
    }

    #[test]
    fn sb_append_double() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendDouble", "()I"),
            4
        );
    }

    #[test]
    fn sb_append_float() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendFloat", "()I"),
            3
        );
    }

    #[test]
    fn sb_init_with_string() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testInitWithString", "()I"),
            8
        );
    }

    #[test]
    fn sb_length() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testLength", "()I"),
            3
        );
    }

    #[test]
    fn sb_loop_build() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testLoopBuild", "()I"),
            5
        );
    }

    #[test]
    fn sb_append_string_object() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendString", "()I"),
            5
        );
    }

    // ---- Character tests ----

    #[test]
    fn char_is_digit() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsDigit", "()I"),
            1
        );
    }

    #[test]
    fn char_is_digit_false() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsDigitFalse", "()I"),
            1
        );
    }

    #[test]
    fn char_is_letter() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetter", "()I"),
            1
        );
    }

    #[test]
    fn char_is_letter_false() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetterFalse", "()I"),
            1
        );
    }

    #[test]
    fn char_is_whitespace() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsWhitespace", "()I"),
            1
        );
    }

    #[test]
    fn char_is_uppercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsUpperCase", "()I"),
            1
        );
    }

    #[test]
    fn char_is_lowercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLowerCase", "()I"),
            1
        );
    }

    #[test]
    fn char_to_uppercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testToUpperCase", "()I"),
            65
        );
    }

    #[test]
    fn char_to_lowercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testToLowerCase", "()I"),
            97
        );
    }

    #[test]
    fn char_is_letter_or_digit() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetterOrDigit", "()I"),
            1
        );
    }

    #[test]
    fn char_valueof_and_charvalue() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testValueOf", "()I"),
            88
        );
    }

    #[test]
    fn arraylist_size() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testSize", "()I"),
            3
        );
    }

    #[test]
    fn arraylist_get() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testGet", "()I"),
            5
        );
    }

    #[test]
    fn arraylist_foreach_count() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testForEachCount", "()I"),
            3
        );
    }

    #[test]
    fn arraylist_foreach_sum() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testForEachSum", "()I"),
            8
        );
    }

    #[test]
    fn arraylist_empty_foreach() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testEmptyForEach", "()I"),
            0
        );
    }

    #[test]
    fn arraylist_single_element() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testSingleElement", "()I"),
            4
        );
    }

    #[test]
    fn arraylist_add_returns_true() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testAddReturnsTrue", "()I"),
            1
        );
    }

    // ---- Phase 22: String.format() integration tests ----

    #[test]
    fn format_string() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatString", "()I"),
            11
        );
    }

    #[test]
    fn format_int() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatInt", "()I"),
            2
        );
    }

    #[test]
    fn format_multiple() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatMultiple", "()I"),
            3
        );
    }

    #[test]
    fn format_double() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatDouble", "()I"),
            4
        );
    }

    #[test]
    fn format_hex() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatHex", "()I"),
            2
        );
    }

    #[test]
    fn format_percent() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatPercent", "()I"),
            4
        );
    }

    #[test]
    fn format_null() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatNull", "()I"),
            4
        );
    }

    #[test]
    fn format_sum() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatSum", "()I"),
            8
        );
    }

    // ---- Phase 22 Task 2: Arrays utilities + numeric constants ----

    #[test]
    fn arrays_fill_int() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testFillInt", "()I"),
            14
        );
    }

    #[test]
    fn arrays_copyof_truncate() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testCopyOfTruncate", "()I"),
            3
        );
    }

    #[test]
    fn arrays_copyof_extend() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testCopyOfExtend", "()I"),
            0
        );
    }

    #[test]
    fn arrays_sort_int() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testSortInt", "()I"),
            19
        );
    }

    #[test]
    fn integer_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testIntegerMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn integer_min_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testIntegerMinValue", "()I"),
            1
        );
    }

    #[test]
    fn long_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testLongMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn double_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testDoubleMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn double_nan() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testDoubleNaN", "()I"),
            1
        );
    }

    // ---- Phase 23 Task 1: HashMap ----

    #[test]
    fn hashmap_put_and_get() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testPutAndGet", "()I"),
            30
        );
    }

    #[test]
    fn hashmap_size() {
        assert_eq!(run_bootstrap_int("HashMapTest.class", "testSize", "()I"), 3);
    }

    #[test]
    fn hashmap_contains_key() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testContainsKey", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_get_missing() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testGetMissing", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_remove() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testRemove", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_is_empty() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testIsEmpty", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_overwrite() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testOverwrite", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_get_or_default() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testGetOrDefault", "()I"),
            106
        );
    }

    #[test]
    fn hashset_add_and_contains() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testAddAndContains", "()I"),
            1
        );
    }

    #[test]
    fn hashset_size() {
        assert_eq!(run_bootstrap_int("HashSetTest.class", "testSize", "()I"), 3);
    }

    #[test]
    fn hashset_no_duplicates() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testNoDuplicates", "()I"),
            2
        );
    }

    #[test]
    fn hashset_remove() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testRemove", "()I"),
            1
        );
    }

    #[test]
    fn hashset_is_empty() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testIsEmpty", "()I"),
            1
        );
    }

    #[test]
    fn hashset_add_returns_false() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testAddReturnsFalse", "()I"),
            1
        );
    }

    #[test]
    fn frame_pool_does_not_change_fib_result() {
        // Regression guard: pool reuse must not corrupt frame state.
        // fib(25) = 75025 — stale locals between pool reuses would produce wrong answer.
        let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(result, 75025);
    }

    #[test]
    fn dispatch_cache_fib_correctness() {
        let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(result, 75025);
    }

    #[test]
    fn dispatch_cache_invokestatic_multiple_methods() {
        let sum = run_bootstrap_int("BenchmarkSuite.class", "benchSum", "()I");
        let fib = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(fib, 75025);
        // benchSum overflows i32: sum(0..499999) = 124999750000 → wraps to 445698416
        assert_eq!(sum, 445698416_i32);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_bytecode_cost_counts_iadd() {
        // benchSum adds integers in a loop 0..500_000 (500_000 iadd ops).
        let (result, registry) = run_fixture("BenchmarkSuite.class", "benchSum", "()I");
        assert_eq!(result, Some(Slot::Int(445_698_416)));
        let stat = &registry.telemetry.bytecode_cost.by_opcode["iadd"];
        assert_eq!(stat.count, 500_000);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_object_lineage_records_allocations() {
        // ForEachTest creates an ArrayList and adds elements.
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        // At least one allocation site must exist (the ArrayList constructor).
        assert!(!registry.telemetry.object_lineage.sites.is_empty());
        // Verify some allocation is attributed to ForEachTest class.
        let has_foreach_alloc = registry
            .telemetry
            .object_lineage
            .sites
            .keys()
            .any(|(cls, _, _)| cls == "ForEachTest");
        assert!(
            has_foreach_alloc,
            "expected at least one allocation from ForEachTest"
        );
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_native_boundary_records_println() {
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        // ForEachTest calls System.out.println which dispatches through println native.
        let stat = registry
            .telemetry
            .native_boundary
            .by_method
            .iter()
            .find(|((_, name), _)| name.contains("println"));
        assert!(
            stat.is_some(),
            "expected println to be recorded in native_boundary"
        );
        let (_, s) = stat.unwrap();
        assert!(s.calls > 0);
        assert_eq!(s.errors, 0);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_exception_flow_records_throw_and_catch() {
        let (result, registry) = run_fixture("ExceptionTest.class", "throwAndCatch", "()I");
        assert_eq!(result, Some(Slot::Int(42)));
        let events = &registry.telemetry.exception_flow.events;
        assert_eq!(events.len(), 1);
        assert!(events[0].exception_class.contains("RuntimeException"));
        assert!(events[0].catch_site.is_some(), "exception should be caught");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_exception_flow_rethrow_caught() {
        let (result, registry) = run_fixture("ExceptionTest.class", "rethrow", "()I");
        assert_eq!(result, Some(Slot::Int(99)));
        let events = &registry.telemetry.exception_flow.events;
        // Two throw events: inner throw + rethrow
        assert!(!events.is_empty());
        assert!(events.iter().all(|e| e.catch_site.is_some()));
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_class_init_dag_records_clinit() {
        // ClinitTest has a static initializer that sets VALUE = 42.
        // Calling getValue() via invokestatic triggers ensure_initialized → <clinit>.
        let (result, registry) = run_fixture("ClinitTest.class", "getValue", "()I");
        assert_eq!(result, Some(Slot::Int(42)));
        let events = &registry.telemetry.class_init_dag.events;
        assert!(
            !events.is_empty(),
            "expected at least one class_init_dag event"
        );
        let ev = events
            .iter()
            .find(|e| e.class == "ClinitTest")
            .expect("expected ClinitTest clinit event");
        assert!(ev.duration_ns > 0, "clinit duration should be positive");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_dispatch_resolution_records_virtual_calls() {
        // ForEachTest calls invokevirtual on ArrayList (add) and invokeinterface
        // for the for-each iterator protocol (iterator, hasNext, next).
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        let dr = &registry.telemetry.dispatch_resolution;
        // At least some virtual/interface dispatch sites must have been recorded.
        assert!(
            !dr.by_site.is_empty(),
            "dispatch_resolution should have entries after ForEachTest"
        );
        // Every recorded site must have at least one call.
        for ((cls, cp), stat) in &dr.by_site {
            assert!(stat.calls > 0, "site {cls}[cp{cp}] should have calls > 0");
        }
    }

    // ---- Phase 24: GC stress tests ----

    #[test]
    fn gc_reclaims_short_lived_objects() {
        // GcStressTest allocates 2000 int[4] arrays in a loop.
        // Verify correct output sum = 0+1+...+1999 = 1999000.
        let result = run_bootstrap_int("GcStressTest.class", "run", "()I");
        assert_eq!(result, 1_999_000);
    }

    #[test]
    fn gc_keeps_heap_bounded() {
        let loader = fixtures_loader();
        let bytes = loader.find_class("GcStressTest").unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        let ctx = build_class_context(&cf);
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut stdout = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut stdout,
            "GcStressTest",
            "run",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1_999_000)));
        // 2000 arrays allocated; GC should have reclaimed most.
        // bootstrap_stdlib pre-populates ~200 permanent live objects (synthetic classes,
        // interned strings, static fields). After GC fires, the 2000 short-lived arrays
        // are collected, so total live count is dominated by bootstrap objects.
        // Without GC the heap would grow to 2000+ objects; with GC it stays bounded.
        let live = heap.len();
        assert!(
            live < 500,
            "heap has {live} live objects — GC may not have fired"
        );
    }

    #[test]
    fn gc_generational_stress_test() {
        let ctx = load_class_context("GcGenerationalStressTest.class");
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Small young_capacity to force frequent minor GCs.
        heap.young_capacity = 32;

        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = duke_runtime::Slot::Reference(None);

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            "main",
            "([Ljava/lang/String;)V",
            &[duke_runtime::Slot::Reference(Some(arr_ref))],
        );
        assert!(
            result.is_ok(),
            "generational GC stress test failed: {result:?}"
        );
        let output = String::from_utf8(out).unwrap();
        // Verify long-lived objects survived all minor GCs.
        for i in 0..10 {
            assert!(
                output.contains(&format!("Survivor-{i}")),
                "long-lived object Survivor-{i} missing from output:\n{output}"
            );
        }
        // sum of "tmp-N".length() for N in 0..5000 = 38890
        assert!(
            output.contains("38890"),
            "expected sum 38890 in output:\n{output}"
        );
    }

    #[test]
    fn callback_handler_is_dispatched_with_invoke_fn() {
        use std::sync::atomic::AtomicBool;
        static CALLED: AtomicBool = AtomicBool::new(false);

        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Simple helper native: returns 42.
        registry
            .natives
            .register("duke/test/Helper", "answer", "()I", |_args, _heap, _out| {
                Ok(Some(Slot::Int(42)))
            });

        // Callback native: invokes the helper and returns its result.
        registry.natives.register_callback(
            "duke/test/Caller",
            "call",
            "()I",
            |_args, heap, output, invoke| {
                CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
                invoke(heap, output, "duke/test/Helper", "answer", "()I", vec![])
            },
        );

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "duke/test/Caller",
            "call",
            "()I",
            &[],
        );
        assert!(result.is_ok(), "callback dispatch failed: {result:?}");
        assert_eq!(result.unwrap(), Some(Slot::Int(42)));
        assert!(
            CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "callback handler was never invoked"
        );
    }

    // ---- Bytecode-level Callback dispatch tests (Sites 2-4) ----
    //
    // These tests verify that `HandlerKind::Callback` handlers fire when the
    // call site is reached via *bytecode* (invokestatic / invokevirtual /
    // invokeinterface), not just via the top-level fast-path.
    //
    // Pattern: bootstrap stdlib (so the fixture class can run), then
    // *override* one specific native with a Callback handler, run the fixture
    // bytecode, and assert both the result and the CALLED flag.

    /// Site 2 — invokestatic Callback arm.
    ///
    /// `ParseArgs.parseInt()` bytecode contains:
    ///   `invokestatic java/lang/Integer.parseInt:(Ljava/lang/String;)I`
    /// We override that registration with a Callback handler that delegates to
    /// `invoke`, proving the arm wires the closure correctly end-to-end.
    #[test]
    fn callback_fires_via_invokestatic_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ParseArgs.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override the Simple Integer.parseInt with a Callback that records
        // invocation and delegates via `invoke` back to the (already-registered)
        // helper that bootstrap_stdlib set up as a Simple handler on
        // "java/lang/Integer"/"parseInt".
        // Because we overwrite the key the Simple handler is gone — we compute
        // the parse directly inside the callback instead.
        registry.natives.register_callback(
            "java/lang/Integer",
            "parseInt",
            "(Ljava/lang/String;)I",
            |args, heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is the String reference; extract its string_value.
                let s = match &args[0] {
                    Slot::Reference(Some(r)) => heap
                        .get(*r)
                        .ok()
                        .and_then(|o| o.string_value)
                        .unwrap_or_default(),
                    _ => return Err(VmError::NullPointerException),
                };
                let n: i32 = s.parse().map_err(|_| VmError::NullPointerException)?;
                Ok(Some(Slot::Int(n)))
            },
        );

        let loader = fixtures_loader();
        // Build String[] = ["123"] for ParseArgs.parseInt
        let s_ref = heap.allocate_string("123".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "parseInt",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        );
        assert!(
            result.is_ok(),
            "invokestatic Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(123)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokestatic bytecode"
        );
    }

    /// Site 3 — invokevirtual Callback arm.
    ///
    /// `ParseArgs.valueOf()` bytecode contains:
    ///   `invokevirtual java/lang/Integer.intValue:()I`
    /// We override that registration with a Callback handler.
    #[test]
    fn callback_fires_via_invokevirtual_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ParseArgs.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override Integer.intValue with a Callback.
        // The Integer heap object stores the boxed int in fields[0].
        registry.natives.register_callback(
            "java/lang/Integer",
            "intValue",
            "()I",
            |args, heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the Integer object); fields[0] holds the int.
                let r = match &args[0] {
                    Slot::Reference(Some(r)) => *r,
                    _ => return Err(VmError::NullPointerException),
                };
                let val = heap.get(r)?.fields[0];
                Ok(Some(val))
            },
        );

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "valueOf",
            "()I",
            &[],
        );
        assert!(
            result.is_ok(),
            "invokevirtual Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(42)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokevirtual bytecode"
        );
    }

    /// Site 4 — invokeinterface Callback arm.
    ///
    /// `ArrayListTest.testForEachCount()` bytecode uses:
    ///   `invokeinterface java/util/Iterator.hasNext:()Z`
    /// dispatched on the actual runtime class `duke/util/ArrayListIterator`.
    /// We override `duke/util/ArrayListIterator.hasNext` with a Callback that
    /// immediately returns false (0), making the for-each body not execute and
    /// the count stay at 0.  This verifies the invokeinterface Callback arm
    /// fires.
    #[test]
    fn callback_fires_via_invokeinterface_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ArrayListTest.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override ArrayListIterator.hasNext with a Callback that records
        // invocation and immediately signals "no more elements" (returns false).
        registry.natives.register_callback(
            "duke/util/ArrayListIterator",
            "hasNext",
            "()Z",
            |_args, _heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                Ok(Some(Slot::Int(0))) // false — loop body never runs
            },
        );

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ArrayListTest",
            "testForEachCount",
            "()I",
            &[],
        );
        assert!(
            result.is_ok(),
            "invokeinterface Callback dispatch failed: {result:?}"
        );
        // hasNext always returns false → loop body never runs → count = 0.
        assert_eq!(result.unwrap(), Some(Slot::Int(0)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokeinterface bytecode"
        );
    }

    /// Site 5 — lambda SAM virtual/interface native-fallback Callback arm.
    ///
    /// `LambdaCallbackTest.capturedLengthViaMethodRef("hello")` compiles to:
    ///
    ///   invokedynamic … get:(Ljava/lang/String;)LLambdaCallbackTest$IntSupplier;
    ///   // creates $$Lambda$0 with impl_class="java/lang/String",
    ///   //   impl_method="length", impl_kind=5 (REF_invokeVirtual),
    ///   //   captured_count=1 (the string "hello")
    ///   invokeinterface LambdaCallbackTest$IntSupplier.get:()I
    ///   // → lambda SAM: impl_kind==5, resolve_method_in_hierarchy returns None
    ///   //   (String has no bytecode methods in Duke), so falls to Site 5:
    ///   //   registry.natives.get_kind("java/lang/String", "length", "()I")
    ///
    /// We override `String.length` with a Callback handler to prove the arm fires.
    #[test]
    fn callback_fires_via_lambda_sam_fallback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("LambdaCallbackTest.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // javac emits `dup; invokestatic Objects.requireNonNull; pop` for
        // captured instance method references.  Register a ClassContext and a
        // passthrough native so the invokestatic dispatch doesn't fail.
        registry.register(ClassContext {
            class_name: "java/util/Objects".to_string(),
            super_class: Some("java/lang/Object".to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
        });
        registry.natives.register(
            "java/util/Objects",
            "requireNonNull",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            |args, _heap, _out| Ok(Some(args[0])),
        );

        // Override String.length with a Callback.  This replaces the Simple
        // handler that bootstrap_stdlib registered, so the lambda SAM fallback
        // (Site 5) must route through the Callback arm to fire at all.
        registry.natives.register_callback(
            "java/lang/String",
            "length",
            "()I",
            |args, heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the captured String reference).
                let r = match &args[0] {
                    Slot::Reference(Some(r)) => *r,
                    _ => return Err(VmError::NullPointerException),
                };
                let len = heap.get(r)?.string_value.as_deref().unwrap_or("").len() as i32;
                Ok(Some(Slot::Int(len)))
            },
        );

        let loader = fixtures_loader();
        let s_ref = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaCallbackTest",
            "capturedLengthViaMethodRef",
            "(Ljava/lang/String;)I",
            &[Slot::Reference(Some(s_ref))],
        );
        assert!(
            result.is_ok(),
            "lambda SAM Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(5)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via lambda SAM fallback (Site 5)"
        );
    }

    #[test]
    fn integer_compare_to_less_returns_negative() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(3);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(5);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn integer_compare_to_equal_returns_zero() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(7);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(7);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    #[test]
    fn string_compare_to_apple_less_than_banana() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate_string("apple".to_string());
        let b = heap.allocate_string("banana".to_string());
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/String",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        match result {
            Some(Slot::Int(n)) => assert!(n < 0, "apple < banana: expected negative, got {n}"),
            other => panic!("expected Int, got {other:?}"),
        }
    }

    #[test]
    fn integer_compare_to_greater_returns_positive() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(9);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(3);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn long_compare_to_less_returns_negative() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Long(100);
        let b = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Long(200);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Long",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn double_compare_to_nan_is_greatest() {
        // Java spec: NaN > any value including POSITIVE_INFINITY.
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Case 1: NaN > +∞ → result should be 1.
        let nan_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(nan_ref).unwrap().fields[0] = Slot::Double(f64::NAN);
        let inf_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(inf_ref).unwrap().fields[0] = Slot::Double(f64::INFINITY);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Double",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[
                Slot::Reference(Some(nan_ref)),
                Slot::Reference(Some(inf_ref)),
            ],
        )
        .unwrap();
        assert_eq!(
            result,
            Some(Slot::Int(1)),
            "NaN.compareTo(+Inf) should be 1 (NaN is greatest)"
        );

        // Case 2: 1.0 < NaN → result should be -1.
        let one_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(one_ref).unwrap().fields[0] = Slot::Double(1.0);
        let nan_ref2 = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(nan_ref2).unwrap().fields[0] = Slot::Double(f64::NAN);
        let mut out2: Vec<u8> = Vec::new();
        let result2 = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out2,
            "java/lang/Double",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[
                Slot::Reference(Some(one_ref)),
                Slot::Reference(Some(nan_ref2)),
            ],
        )
        .unwrap();
        assert_eq!(
            result2,
            Some(Slot::Int(-1)),
            "1.0.compareTo(NaN) should be -1 (NaN is greatest)"
        );
    }

    // ---- Phase 26 Task 4: ArrayList.sort(Comparator) via CallbackNativeHandler ----

    #[test]
    fn array_list_sort_integers_via_callback() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Build ArrayList [Integer(3), Integer(1), Integer(4)]
        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3); // size
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let i3 = make_int(&mut heap, 3);
        let i1 = make_int(&mut heap, 1);
        let i4 = make_int(&mut heap, 4);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i3));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i1));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i4));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)], // null Comparator
        )
        .unwrap();

        // After sort: fields[1..=3] = Integer(1), Integer(3), Integer(4)
        let val = |heap: &duke_gc::Heap, s: &Slot| -> i32 {
            match s {
                Slot::Reference(Some(r)) => match heap.get(*r).unwrap().fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        let f = |i: usize| heap.get(list).unwrap().fields[i];
        assert_eq!(val(&heap, &f(1)), 1);
        assert_eq!(val(&heap, &f(2)), 3);
        assert_eq!(val(&heap, &f(3)), 4);
    }

    #[test]
    fn array_list_sort_strings_via_callback() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
        let sb = heap.allocate_string("banana".to_string());
        let sa = heap.allocate_string("apple".to_string());
        let sc = heap.allocate_string("cherry".to_string());
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(sb));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(sa));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(sc));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let str_val = |heap: &duke_gc::Heap, s: &Slot| -> String {
            match s {
                Slot::Reference(Some(r)) => heap
                    .get(*r)
                    .unwrap()
                    .string_value

                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        let f = |i: usize| heap.get(list).unwrap().fields[i];
        assert_eq!(str_val(&heap, &f(1)), "apple");
        assert_eq!(str_val(&heap, &f(2)), "banana");
        assert_eq!(str_val(&heap, &f(3)), "cherry");
    }

    // Fix 6: boundary tests for array_list_sort

    #[test]
    fn array_list_sort_empty_list_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // ArrayList with size=0 — allocate just the size field slot.
        let list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        );
        assert!(result.is_ok(), "empty sort should not error: {result:?}");
        assert_eq!(result.unwrap(), None);
        // Size field still 0.
        assert_eq!(heap.get(list).unwrap().fields[0], Slot::Int(0));
    }

    #[test]
    fn array_list_sort_single_element_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 2);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(1);
        let elem = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(elem).unwrap().fields[0] = Slot::Int(42);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(elem));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        // Single element unchanged.
        match &heap.get(list).unwrap().fields[1] {
            Slot::Reference(Some(r)) => {
                let r = *r;
                assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
            }
            other => panic!("unexpected slot: {other:?}"),
        }
    }

    #[test]
    fn array_list_sort_already_sorted_unchanged() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let i1 = make_int(&mut heap, 1);
        let i2 = make_int(&mut heap, 2);
        let i3 = make_int(&mut heap, 3);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i1));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i2));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i3));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
            match heap.get(list).unwrap().fields[i] {
                Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                    Slot::Int(n) => n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        assert_eq!(int_val(&heap, 1), 1);
        assert_eq!(int_val(&heap, 2), 2);
        assert_eq!(int_val(&heap, 3), 3);
    }

    #[test]
    fn array_list_sort_duplicates() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // [3, 1, 1, 2] → [1, 1, 2, 3]
        let list = heap.allocate("java/util/ArrayList".to_string(), 5);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(4);
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let r3 = make_int(&mut heap, 3);
        let r1a = make_int(&mut heap, 1);
        let r1b = make_int(&mut heap, 1);
        let r2 = make_int(&mut heap, 2);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(r3));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(r1a));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(r1b));
        heap.get_mut(list).unwrap().fields[4] = Slot::Reference(Some(r2));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
            match heap.get(list).unwrap().fields[i] {
                Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                    Slot::Int(n) => n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        assert_eq!(int_val(&heap, 1), 1);
        assert_eq!(int_val(&heap, 2), 1);
        assert_eq!(int_val(&heap, 3), 2);
        assert_eq!(int_val(&heap, 4), 3);
    }

    // ---- Phase 26 Task 5: CollectionsSortTest end-to-end integration test ----

    #[test]
    fn collections_sort_end_to_end() {
        let ctx = load_class_context("CollectionsSortTest.class");
        let entry_class = ctx.class_name;
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let loader = fixtures_loader();
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            "main",
            "([Ljava/lang/String;)V",
            &[Slot::Reference(Some(arr_ref))],
        );
        assert!(result.is_ok(), "CollectionsSortTest failed: {result:?}");
        let output = String::from_utf8(out).unwrap();
        // Integer sort: 1 1 3 4 5 (one per line)
        assert!(
            output.contains("1\n1\n3\n4\n5"),
            "integer sort wrong:\n{output}"
        );
        // String sort: apple banana cherry (one per line)
        assert!(
            output.contains("apple\nbanana\ncherry"),
            "string sort wrong:\n{output}"
        );
    }

    #[test]
    fn collections_sort_null_list_raises_npe() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/Collections",
            "sort",
            "(Ljava/util/List;)V",
            &[Slot::Reference(None)],
        );
        assert!(
            matches!(result, Err(VmError::NullPointerException)),
            "expected NullPointerException, got {result:?}"
        );
    }

    #[test]
    fn collections_sort_empty_list_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Empty ArrayList: size=0
        let list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/Collections",
            "sort",
            "(Ljava/util/List;)V",
            &[Slot::Reference(Some(list))],
        );
        assert!(result.is_ok(), "empty list sort failed: {result:?}");
    }

    // --- Phase 27: try-with-resources ---

    #[test]
    fn try_with_resources_simple_value() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "simpleValue", "()I"),
            42
        );
    }

    #[test]
    fn try_with_resources_closed_on_success() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "closedOnSuccess", "()I"),
            1
        );
    }

    #[test]
    fn try_with_resources_closed_on_exception() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "closedOnException", "()I"),
            1
        );
    }

    #[test]
    fn try_with_resources_nested_closed() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "nestedClosed", "()I"),
            2
        );
    }
}
