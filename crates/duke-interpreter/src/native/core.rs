static NEXT_ZIP_ID: AtomicI32 = AtomicI32::new(100_000_000);
macro_rules! extract_print_arg {
    ($args:expr, $pat:pat => $expr:expr, $expected:literal) => {
        match $args .get(1) { Some($pat) => $expr, _ => { return Err(Error::TypeMismatch
        { expected : $expected, got : "other", }); } }
    };
}
const JUL_LEVEL_VALUE_FIELD: usize = 0;
const JUL_LOGGER_NAME_FIELD: usize = 0;
const JUL_LOGGER_LEVEL_FIELD: usize = 1;
const JUL_LOGGER_USE_PARENT_HANDLERS_FIELD: usize = 2;
const JUL_LOGGER_PARENT_FIELD: usize = 3;
const JUL_LOGGER_HANDLER_COUNT_FIELD: usize = 4;
const JUL_LOGGER_HANDLERS_START: usize = 5;
const JUL_HANDLER_LEVEL_FIELD: usize = 0;
const JUL_HANDLER_FORMATTER_FIELD: usize = 1;
const JUL_MANAGER_ROOT_LOGGER_FIELD: usize = 0;
const JUL_MANAGER_LOGGER_COUNT_FIELD: usize = 1;
const JUL_MANAGER_LOGGERS_START: usize = 2;
const JUL_LOG_RECORD_LEVEL_FIELD: usize = 0;
const JUL_LOG_RECORD_MESSAGE_FIELD: usize = 1;
const JUL_LOG_RECORD_LOGGER_NAME_FIELD: usize = 2;
const JUL_LOG_RECORD_THROWN_FIELD: usize = 3;
const JUL_LOG_RECORD_MILLIS_FIELD: usize = 4;
const JUL_LOG_RECORD_PARAMETERS_FIELD: usize = 5;
const JUL_ENUM_INDEX_FIELD: usize = 0;
const JUL_ENUM_COUNT_FIELD: usize = 1;
const JUL_ENUM_NAMES_START: usize = 2;
const JUL_LEVELS: [(&str, i32); 9] = [
    ("SEVERE", 1000),
    ("WARNING", 900),
    ("INFO", 800),
    ("CONFIG", 700),
    ("FINE", 500),
    ("FINER", 400),
    ("FINEST", 300),
    ("ALL", i32::MIN),
    ("OFF", i32::MAX),
];
const REPLACEMENT_CHAR: char = '\u{fffd}';
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StandardCharset {
    Utf8,
    Utf16,
    Utf16Be,
    Utf16Le,
    UsAscii,
    Iso88591,
}
impl StandardCharset {
    const fn canonical_name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16 => "UTF-16",
            Self::Utf16Be => "UTF-16BE",
            Self::Utf16Le => "UTF-16LE",
            Self::UsAscii => "US-ASCII",
            Self::Iso88591 => "ISO-8859-1",
        }
    }
    fn from_canonical(name: &str) -> Option<Self> {
        match name {
            "UTF-8" => Some(Self::Utf8),
            "UTF-16" => Some(Self::Utf16),
            "UTF-16BE" => Some(Self::Utf16Be),
            "UTF-16LE" => Some(Self::Utf16Le),
            "US-ASCII" => Some(Self::UsAscii),
            "ISO-8859-1" => Some(Self::Iso88591),
            _ => None,
        }
    }
}
#[derive(Clone, Copy)]
enum Utf16Endian {
    Big,
    Little,
}
const BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT: usize = 1;
const BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT: usize = 0;
const BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT: usize = 2;
const THROWABLE_CAUSE_FIELD: usize = 0;
const THROWABLE_STACK_TRACE_FIELD: usize = 1;
const THROWABLE_SUPPRESSED_FIELD: usize = 2;
const STACK_TRACE_ELEMENT_CLASS: &str = "java/lang/StackTraceElement";
const STACK_TRACE_ARRAY_CLASS: &str = "[Ljava/lang/StackTraceElement;";
const THROWABLE_ARRAY_CLASS: &str = "[Ljava/lang/Throwable;";
enum KeyOrd {
    Int(i64),
    Flt(f64),
    Str(String),
}
#[derive(PartialEq)]
enum TreeSortKey {
    Num(f64),
    Str(String),
}
impl TreeSortKey {
    fn less_than(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a < b,
            (Self::Str(a), Self::Str(b)) => a < b,
            _ => false,
        }
    }
}
const RESOURCE_STREAM_BYTES_FIELD: usize = 0;
const RESOURCE_STREAM_CURSOR_FIELD: usize = 1;
const RESOURCE_STREAM_CLOSED_FIELD: usize = 2;
const RESOURCE_ENUM_INDEX_FIELD: usize = 0;
const RESOURCE_ENUM_COUNT_FIELD: usize = 1;
const RESOURCE_ENUM_VALUES_START: usize = 2;
const BOOT_ARCHIVE_ENTRY_NAME_SLOT: usize = 0;
const BOOT_ARCHIVE_ENTRY_DIRECTORY_SLOT: usize = 1;
const ANNOTATION_PROXY_PREFIX: &str = "duke/annotation/AnnotationProxy:";
static SYSTEM_PROPERTY_OVERRIDES: std::sync::OnceLock<
    std::sync::Mutex<HashMap<String, String>>,
> = std::sync::OnceLock::new();
static NANO_TIME_ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
const THREAD_TARGET_SLOT: usize = 0;
const THREAD_ID_SLOT: usize = 1;
const THREAD_INTERRUPTED_SLOT: usize = 2;
const THREAD_HOST_KEY_SLOT: usize = 3;
static NEXT_THREAD_HOST_KEY: AtomicI32 = AtomicI32::new(1);
const EXECUTOR_SHUTDOWN_FIELD: usize = 0;
const EXECUTOR_AWAIT_DEADLINE_FIELD: usize = 1;
const FUTURE_STATE_FIELD: usize = 0;
const FUTURE_RESULT_FIELD: usize = 1;
const FUTURE_EXCEPTION_FIELD: usize = 2;
const FUTURE_WAIT_DEADLINE_FIELD: usize = 3;
const FUTURE_TASK_FIELD: usize = 4;
const FUTURE_PENDING: i32 = 0;
const FUTURE_RUNNING: i32 = 1;
const FUTURE_DONE: i32 = 2;
const FUTURE_CANCELLED: i32 = 3;
const FUTURE_FAILED: i32 = 4;
const TIMEUNIT_NANOS_FIELD: usize = 2;
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";
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
    const fn new() -> Self {
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
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}
/// Pre-resolved method dispatch entry — cached on first resolution to eliminate
/// repeated [`ClassRegistry`] `HashMap` lookups on hot call sites.
///
/// Stored in the static dispatch cache (`dispatch_cache`) and the virtual
/// dispatch cache (`vtable_cache`).  All `Arc` fields are cheap to clone
/// (reference-count bump only).
///
/// The caches are keyed as follows:
/// - `dispatch_cache`: `(caller_class, cp_idx)` for `invokestatic`/`invokespecial`
/// - `vtable_cache`: `(caller_class, cp_idx, receiver_runtime_class)` for `invokevirtual`
struct CachedDispatch {
    class_name: String,
    method_idx: usize,
    arg_count: usize,
    /// JVM primitive type chars for each parameter ('I', 'J', 'D', 'F', 'Z', 'B', 'C', 'S', 'L', '[').
    /// Used to assign wide types (J/D) to the correct local variable slots.
    param_types: Vec<char>,
    max_locals: usize,
    max_stack: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
}
struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    /// Static dispatch cache for `invokestatic` and `invokespecial`.
    /// Key: (`caller_class`, `cp_idx`) → pre-resolved method data.
    dispatch_cache: HashMap<String, HashMap<u16, CachedDispatch>>,
    /// Polymorphic inline cache for `invokevirtual`.
    /// Key: (`caller_class`, `cp_idx`, `receiver_runtime_class`) → pre-resolved method data.
    vtable_cache: HashMap<String, HashMap<u16, HashMap<String, CachedDispatch>>>,
    idx: usize,
    string_intern: HashMap<(u8, String), u64>,
    #[cfg(feature = "telemetry")]
    current_method: String,
}
struct InterpreterCallbackOps<'a> {
    registry: &'a mut ClassRegistry,
    loader: &'a dyn ClassLoader,
}
impl CallbackOps for InterpreterCallbackOps<'_> {
    fn invoke(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
        method: &str,
        descriptor: &str,
        args: Vec<Slot>,
    ) -> Result<Option<Slot>> {
        if let Some(result) = callback_invoke_registered_lambda(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )? {
            return Ok(result);
        }
        execute_class(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )
    }
    fn ensure_loaded(&mut self, class: &str) -> Result<()> {
        match self.registry.resolve_loaded_class_key(class) {
            Ok(_) => return Ok(()),
            Err(Error::ClassNotFound { .. }) => {}
            Err(err) => return Err(err),
        }
        if self.registry.ensure_loaded(class, self.loader)? {
            Ok(())
        } else {
            Err(Error::ClassNotFound {
                name: class.to_string(),
            })
        }
    }
    fn ensure_parent_loaded(&mut self, class: &str) -> Result<Option<String>> {
        if self.registry.contains(class) {
            return Ok(Some(class.to_string()));
        }
        if self.registry.ensure_loaded(class, self.loader)?
            && self.registry.contains(class)
        {
            Ok(Some(class.to_string()))
        } else {
            Ok(None)
        }
    }
    fn inspect_class(&mut self, class: &str) -> Result<ReflectedClassInfo> {
        inspect_reflected_class(self.registry, self.loader, class)
    }
    fn ensure_class_initialized(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> Result<()> {
        self.ensure_loaded(class)?;
        ensure_initialized(self.registry, self.loader, heap, output, class, "")
    }
    fn code_source_for_class(&mut self, class: &str) -> Result<Option<String>> {
        Ok(self.registry.code_source_for_class(class).map(ToOwned::to_owned))
    }
    fn ensure_loaded_with_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> Result<()> {
        let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
        for path in paths {
            if self
                .registry
                .ensure_loaded_with_provenance(class, &path, Some(loader_ref))?
            {
                return Ok(());
            }
        }
        self.ensure_loaded(class)
    }
    fn instance_field_slot(&mut self, class: &str, field_name: &str) -> Result<usize> {
        field_slot_idx(self.registry, class, field_name)
    }
    fn read_instance_field(
        &mut self,
        heap: &duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
    ) -> Result<Slot> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(Error::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        Ok(heap.get(object_ref)?.fields[slot])
    }
    fn write_instance_field(
        &mut self,
        heap: &mut duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
        value: Slot,
    ) -> Result<()> {
        let actual_class = heap.get(object_ref)?.class_name.clone();
        if !is_assignable_from(
            self.registry,
            self.loader,
            &actual_class,
            declaring_class,
            Some(declaring_class),
        ) {
            return Err(Error::JavaException {
                class_name: "java/lang/IllegalArgumentException".to_string(),
            });
        }
        let slot = field_slot_idx(self.registry, declaring_class, field_name)?;
        heap.write_field(object_ref, slot, value)?;
        Ok(())
    }
    fn read_static_field(&mut self, class: &str, field_name: &str) -> Result<Slot> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        Ok(self.registry.get(class)?.static_fields[slot])
    }
    fn write_static_field(
        &mut self,
        class: &str,
        field_name: &str,
        value: Slot,
    ) -> Result<()> {
        let slot = {
            let ctx = self.registry.get(class)?;
            static_field_idx(ctx, field_name)?
        };
        self.registry.get_mut(class)?.static_fields[slot] = value;
        Ok(())
    }
    fn runtime_loader_for_class(&mut self, class: &str) -> Result<Option<u64>> {
        Ok(self.registry.runtime_loader_for_class(class))
    }
    fn service_configuration_files(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        service_binary_name: &str,
    ) -> Result<Vec<Vec<u8>>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self
                .registry
                .service_configuration_files_for_paths(&paths, service_binary_name);
        }
        self.loader
            .service_configuration_files(service_binary_name)
            .map_err(|_| Error::JavaException {
                class_name: "java/util/ServiceConfigurationError".to_string(),
            })
    }
    fn find_resource_entry(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        name: &str,
    ) -> Result<Option<duke_loader::LocatedResource>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self.registry.find_resource_entry_for_paths(&paths, name);
        }
        match self.loader.find_resource_entry(name) {
            Ok(resource) => Ok(Some(resource)),
            Err(duke_loader::Error::NotFound { .. }) => Ok(None),
            Err(_) => {
                Err(Error::JavaException {
                    class_name: "java/io/IOException".to_string(),
                })
            }
        }
    }
    fn find_resource_entries(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: Option<u64>,
        name: &str,
    ) -> Result<Vec<duke_loader::LocatedResource>> {
        if let Some(loader_ref) = loader_ref {
            let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
            return self.registry.find_resource_entries_for_paths(&paths, name);
        }
        self.loader
            .find_resource_entries(name)
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })
    }
    fn class_key_for_loaded_class(&mut self, class: &str) -> Result<String> {
        self.registry.resolve_loaded_class_key(class)
    }
    fn class_key_for_runtime_loader(
        &mut self,
        heap: &duke_gc::Heap,
        loader_ref: u64,
        class: &str,
    ) -> Result<String> {
        let paths = runtime_loader_paths(self.registry, heap, loader_ref)?;
        if let Some(path) = paths.first() {
            return Ok(
                self
                    .registry
                    .class_key_from_provenance(class, Some(path), Some(loader_ref)),
            );
        }
        self.class_key_for_loaded_class(class)
    }
    fn class_key_from_source(
        &mut self,
        class: &str,
        source_class: Option<&str>,
    ) -> Result<String> {
        Ok(self.registry.class_key_from_source(class, source_class))
    }
    fn allocate_instance(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
    ) -> Result<u64> {
        allocate_reflection_instance(self.registry, self.loader, heap, output, class)
    }
}
impl ExecutionState {
    fn new(
        registry: &ClassRegistry,
        class_name: &str,
        #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))]
        method_name: &str,
        entry_idx: usize,
        args: &[Slot],
    ) -> Result<Self> {
        let current_class = class_name.to_string();
        let pc_to_idx = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].pc_to_idx)
        };
        let frame = {
            let ctx = registry.get(&current_class)?;
            Frame::new(
                usize::from(ctx.methods[entry_idx].max_stack),
                usize::from(ctx.methods[entry_idx].max_locals),
                args.to_vec(),
            )?
        };
        let instructions = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].instructions)
        };
        Ok(Self {
            current_class,
            method_idx: entry_idx,
            pc_to_idx,
            instructions,
            frame,
            call_stack: Vec::with_capacity(32),
            frame_pool: FramePool::new(),
            dispatch_cache: HashMap::new(),
            vtable_cache: HashMap::new(),
            idx: 0,
            string_intern: HashMap::new(),
            #[cfg(feature = "telemetry")]
            current_method: method_name.to_string(),
        })
    }
}
/// Swap interpreter state to begin executing a callee method.
///
/// All method data is passed pre-resolved so this function performs **zero**
/// [`ClassRegistry`] lookups — eliminating the registry `HashMap` access from every
/// method-dispatch hot path.
/// Maximum Java call stack depth before raising `StackOverflowError`.
const MAX_CALL_DEPTH: usize = 500;
enum ExecutionOutcome {
    Returned(Option<Slot>),
    ThreadAction(NativeThreadAction),
    /// The thread has exhausted its instruction quantum and should yield so
    /// other threads can make progress.
    Yield,
}
/// Default number of bytecode instructions a thread may execute before
/// yielding the VM lock, enabling fair interleaving of Java threads.
const DEFAULT_THREAD_QUANTUM: usize = 1024;
struct CompletionVm {
    registry: ClassRegistry,
    heap: duke_gc::Heap,
    output: Vec<u8>,
    live_workers: usize,
}
#[derive(Default)]
struct CompletionRuntime {
    threads: threading::ThreadRuntime,
    handles: HashMap<i32, std::thread::JoinHandle<Result<()>>>,
    next_executor_worker_id: i32,
    executor_handles: HashMap<i32, std::thread::JoinHandle<Result<()>>>,
}
struct ExecutorInvocation {
    state: ExecutionState,
    impl_desc: Option<String>,
    sam_desc: Option<String>,
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
const REFLECTION_MEMBER_DECLARING_CLASS_FIELD: usize = 0;
const REFLECTION_MEMBER_NAME_FIELD: usize = 1;
const REFLECTION_MEMBER_DESCRIPTOR_FIELD: usize = 2;
const REFLECTION_MEMBER_PUBLIC_FIELD: usize = 3;
const REFLECTION_MEMBER_STATIC_FIELD: usize = 4;
const REFLECTION_MEMBER_ACCESSIBLE_FIELD: usize = 5;
struct ReflectedFieldHandle {
    declaring_class_key: String,
    field_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}
struct ReflectedMethodHandle {
    declaring_class_key: String,
    method_name: String,
    descriptor: String,
    is_public: bool,
    is_static: bool,
    is_accessible: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum MethodHierarchyLookup {
    Bytecode(String, usize),
    NativeOverride,
    Missing,
}
type PendingExceptionMessages = std::sync::Mutex<HashMap<String, VecDeque<String>>>;
type PendingExceptionCauses = std::sync::Mutex<HashMap<String, VecDeque<Slot>>>;
type UncaughtExceptionRefs = std::sync::Mutex<
    HashMap<std::thread::ThreadId, VecDeque<(String, u64)>>,
>;
const SERVICE_LOADER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_LOADER_LOADER_FIELD: usize = 1;
const SERVICE_LOADER_COUNT_FIELD: usize = 2;
const SERVICE_LOADER_PROVIDERS_START: usize = 3;
const SERVICE_ITER_SERVICE_CLASS_FIELD: usize = 0;
const SERVICE_ITER_LOADER_FIELD: usize = 1;
const SERVICE_ITER_INDEX_FIELD: usize = 2;
const SERVICE_ITER_COUNT_FIELD: usize = 3;
const SERVICE_ITER_PROVIDERS_START: usize = 4;
#[allow(clippy::unreadable_literal)]
const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_ADDEND: u64 = 0xB;
const RANDOM_MASK: u64 = (1u64 << 48) - 1;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Base64Variant {
    Standard,
    Mime,
    Url,
}
impl Base64Variant {
    const fn field_value(self) -> i32 {
        match self {
            Self::Standard => 0,
            Self::Mime => 1,
            Self::Url => 2,
        }
    }
    const fn from_field(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Standard),
            1 => Some(Self::Mime),
            2 => Some(Self::Url),
            _ => None,
        }
    }
    const fn alphabet(self) -> &'static [u8; 64] {
        match self {
            Self::Standard | Self::Mime => {
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            }
            Self::Url => {
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
            }
        }
    }
}
const DUKE_SECURITY_PROVIDER: &str = "DUKE";
const MESSAGE_DIGEST_SERVICE_TYPE: &str = "MessageDigest";
const MESSAGE_DIGEST_ALGORITHMS: [&str; 3] = ["SHA-256", "SHA-1", "MD5"];
const MESSAGE_DIGEST_ALGORITHM_FIELD: usize = 0;
const MESSAGE_DIGEST_BUFFER_FIELD: usize = 1;
const PROVIDER_SERVICE_PROVIDER_FIELD: usize = 0;
const PROVIDER_SERVICE_TYPE_FIELD: usize = 1;
const PROVIDER_SERVICE_ALGORITHM_FIELD: usize = 2;
const UUID_MSB_FIELD: usize = 0;
const UUID_LSB_FIELD: usize = 1;
const PATTERN_UNIX_LINES: i32 = 1;
const PATTERN_CASE_INSENSITIVE: i32 = 2;
const PATTERN_COMMENTS: i32 = 4;
const PATTERN_MULTILINE: i32 = 8;
const PATTERN_LITERAL: i32 = 16;
const PATTERN_DOTALL: i32 = 32;
const PATTERN_UNICODE_CASE: i32 = 64;
const PATTERN_CANON_EQ: i32 = 128;
const PATTERN_UNICODE_CHARACTER_CLASS: i32 = 256;
const PATTERN_SUPPORTED_FLAGS: i32 = PATTERN_UNIX_LINES | PATTERN_CASE_INSENSITIVE
    | PATTERN_COMMENTS | PATTERN_MULTILINE | PATTERN_LITERAL | PATTERN_DOTALL
    | PATTERN_UNICODE_CASE | PATTERN_CANON_EQ | PATTERN_UNICODE_CHARACTER_CLASS;
const PATTERN_FLAGS_FIELD: usize = 0;
const MATCHER_PATTERN_FIELD: usize = 0;
const MATCHER_INPUT_FIELD: usize = 1;
const MATCHER_POS_FIELD: usize = 2;
const MATCHER_MATCH_START_FIELD: usize = 3;
const MATCHER_MATCH_END_FIELD: usize = 4;
const MATCHER_APPEND_POS_FIELD: usize = 5;
const MATCHER_FIELD_COUNT: usize = 6;
#[derive(Clone, Copy)]
enum MatcherGroup<'a> {
    Index(usize),
    Name(&'a str),
}
const PROPERTIES_SIZE_FIELD: usize = 0;
const PROPERTIES_DEFAULTS_FIELD: usize = 1;
const PROPERTIES_ENTRIES_START: usize = 2;
const PROPERTIES_ENUM_INDEX_FIELD: usize = 0;
const PROPERTIES_ENUM_COUNT_FIELD: usize = 1;
const PROPERTIES_ENUM_NAMES_START: usize = 2;
const PROPERTIES_STORE_TIMESTAMP: &str = "1970-01-01T00:00:00Z";
const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;
const NANOS_PER_SECOND_I128: i128 = 1_000_000_000;
const NANOS_PER_MILLI_I128: i128 = 1_000_000;
const SECONDS_PER_DAY_I64: i64 = 86_400;
/// Helper: dispatch `comparator.compare(a, b)` via ops.invoke.
fn invoke_comparator(
    comparator: Slot,
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<i32> {
    let Slot::Reference(Some(cmp_ref)) = comparator else {
        return Ok(0);
    };
    let cmp_class = heap.get(cmp_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &cmp_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![comparator, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(
        match res {
            Slot::Int(n) => n,
            _ => 0,
        },
    )
}
/// Helper: dispatch `predicate.test(elem)` via ops.invoke, returns bool.
fn invoke_predicate_test(
    predicate: Slot,
    elem: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<bool> {
    let Slot::Reference(Some(pred_ref)) = predicate else {
        return Ok(false);
    };
    let pred_class = heap.get(pred_ref)?.class_name.clone();
    let res = ops
        .invoke(
            heap,
            out,
            &pred_class,
            "test",
            "(Ljava/lang/Object;)Z",
            vec![predicate, elem],
        )?
        .unwrap_or(Slot::Int(0));
    Ok(matches!(res, Slot::Int(n) if n != 0))
}
fn invoke_consumer_accept(
    consumer: Slot,
    arg: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<()> {
    let Slot::Reference(Some(c_ref)) = consumer else {
        return Ok(());
    };
    let c_class = heap.get(c_ref)?.class_name.clone();
    ops.invoke(
        heap,
        out,
        &c_class,
        "accept",
        "(Ljava/lang/Object;)V",
        vec![consumer, arg],
    )?;
    Ok(())
}
/// Helper: dispatch `function.apply(input)` via ops.invoke.
fn invoke_function_apply(
    function: Slot,
    input: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<Slot> {
    let Slot::Reference(Some(fn_ref)) = function else {
        return Ok(Slot::Reference(None));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    Ok(
        ops
            .invoke(
                heap,
                out,
                &fn_class,
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
                vec![function, input],
            )?
            .unwrap_or(Slot::Reference(None)),
    )
}
fn enqueue_executor_task(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    executor_ref: u64,
    future_ref: u64,
    task_ref: u64,
    kind: duke_gc::ExecutorTaskKind,
) -> Result<()> {
    let executor = {
        let shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        executor_shared(&shared_guard.heap, executor_ref)?
    };
    let mut workers_to_spawn = 0usize;
    {
        let mut guard = executor
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.shutdown {
            return Ok(());
        }
        guard
            .queue
            .push_back(duke_gc::ExecutorTask {
                future_ref,
                task_ref,
                kind,
            });
        while guard.workers < guard.max_workers
            && guard.workers < guard.queue.len().saturating_add(guard.active)
        {
            guard.workers = guard.workers.saturating_add(1);
            workers_to_spawn = workers_to_spawn.saturating_add(1);
        }
        drop(guard);
    }
    for _ in 0..workers_to_spawn {
        {
            let mut shared_guard = shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared_guard.live_workers = shared_guard.live_workers.saturating_add(1);
        }
        spawn_executor_worker(std::sync::Arc::clone(&executor), shared, runtime, loader);
    }
    executor.available.notify_all();
    Ok(())
}
fn date_time_parse_error(input: &str) -> Error {
    push_pending_java_exception_message(
        "java/time/format/DateTimeParseException",
        format!("Text '{input}' could not be parsed"),
    );
    Error::JavaException {
        class_name: "java/time/format/DateTimeParseException".to_string(),
    }
}
fn collect_public_reflected_fields(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> Result<Vec<(String, ReflectedFieldInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_fields_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}
fn collect_public_reflected_fields_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_fields: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedFieldInfo)>,
) -> Result<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for field in reflected.fields {
        if !field.is_public {
            continue;
        }
        let key = format!("{class}\0{}\0{}", field.name, field.descriptor);
        if seen_fields.insert(key) {
            collected.push((class.to_string(), field));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_fields_inner(
            ops,
            &super_class,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_fields_inner(
            ops,
            &interface,
            visited_classes,
            seen_fields,
            collected,
        )?;
    }
    Ok(())
}
fn collect_public_reflected_methods(
    ops: &mut dyn CallbackOps,
    class: &str,
) -> Result<Vec<(String, ReflectedMethodInfo)>> {
    let mut collected = Vec::new();
    collect_public_reflected_methods_inner(
        ops,
        class,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut collected,
    )?;
    Ok(collected)
}
fn collect_public_reflected_methods_inner(
    ops: &mut dyn CallbackOps,
    class: &str,
    visited_classes: &mut HashSet<String>,
    seen_methods: &mut HashSet<String>,
    collected: &mut Vec<(String, ReflectedMethodInfo)>,
) -> Result<()> {
    if !visited_classes.insert(class.to_string()) {
        return Ok(());
    }
    let reflected = ops.inspect_class(class)?;
    for method in reflected.methods {
        if !method.is_public || method.name == "<init>" || method.name == "<clinit>" {
            continue;
        }
        let key = format!(
            "{}\0{}", method.name, descriptor_parameter_part(& method.descriptor)
        );
        if seen_methods.insert(key) {
            collected.push((class.to_string(), method));
        }
    }
    if let Some(super_class) = reflected.super_class {
        collect_public_reflected_methods_inner(
            ops,
            &super_class,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    for interface in reflected.interfaces {
        collect_public_reflected_methods_inner(
            ops,
            &interface,
            visited_classes,
            seen_methods,
            collected,
        )?;
    }
    Ok(())
}
/// Native: `Consumer.andThen(Consumer)Consumer` — chains two consumers sequentially.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_consumer_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndThenConsumer".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}
#[allow(clippy::too_many_arguments)]
fn native_control_for_call(
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
    bci: usize,
    call_stack: &[CallFrame],
    native_class: &str,
    native_method: &str,
    native_desc: &str,
) -> NativeControl {
    let mut control = NativeControl::default();
    if native_needs_stack_snapshot(registry, native_class, native_method, native_desc) {
        control
            .set_stack_trace(
                capture_stack_trace_snapshot(
                    registry,
                    current_class,
                    method_idx,
                    bci,
                    call_stack,
                ),
            );
    }
    control
}
fn parameter_descriptor_from_class_array(
    heap: &duke_gc::Heap,
    slot: Slot,
) -> Result<String> {
    let params = reflection_array_elements(heap, slot)?;
    let mut descriptor = String::from("(");
    for param in params {
        let class_ref = match param {
            Slot::Reference(Some(class_ref)) => class_ref,
            Slot::Reference(None) => return Err(Error::NullPointerException),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "reference array",
                    got: "other",
                });
            }
        };
        descriptor.push_str(&class_descriptor_from_class_ref(heap, class_ref)?);
    }
    descriptor.push(')');
    Ok(descriptor)
}
fn unbox_reflection_argument(
    heap: &duke_gc::Heap,
    descriptor: char,
    arg: Slot,
) -> Result<Slot> {
    match descriptor {
        'L' | '[' => {
            match arg {
                Slot::Reference(_) => Ok(arg),
                _ => {
                    Err(Error::TypeMismatch {
                        expected: "reference",
                        got: "other",
                    })
                }
            }
        }
        'B' | 'C' | 'I' | 'S' | 'Z' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Int(value)) => Ok(Slot::Int(*value)),
                _ => {
                    Err(Error::TypeMismatch {
                        expected: "boxed int-like primitive",
                        got: "other",
                    })
                }
            }
        }
        'J' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Long(value)) => Ok(Slot::Long(*value)),
                _ => {
                    Err(Error::TypeMismatch {
                        expected: "boxed long",
                        got: "other",
                    })
                }
            }
        }
        'F' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Float(value)) => Ok(Slot::Float(*value)),
                _ => {
                    Err(Error::TypeMismatch {
                        expected: "boxed float",
                        got: "other",
                    })
                }
            }
        }
        'D' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(Error::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Double(value)) => Ok(Slot::Double(*value)),
                _ => {
                    Err(Error::TypeMismatch {
                        expected: "boxed double",
                        got: "other",
                    })
                }
            }
        }
        _ => {
            Err(Error::Unimplemented {
                mnemonic: "reflection primitive unboxing",
            })
        }
    }
}
fn exploded_archive_relative_entry_name(
    root_path: &std::path::Path,
    entry_path: &std::path::Path,
    is_directory: bool,
) -> String {
    let mut relative = entry_path
        .strip_prefix(root_path)
        .unwrap_or(entry_path)
        .to_string_lossy()
        .replace('\\', "/");
    if is_directory && !relative.ends_with('/') {
        relative.push('/');
    }
    relative
}
fn load_atomic_reference(heap: &duke_gc::Heap, this_ref: u64) -> Result<Slot> {
    with_atomic_reference(
        heap,
        this_ref,
        |cell| { Ok(*cell.lock().unwrap_or_else(std::sync::PoisonError::into_inner)) },
    )
}
fn broken_barrier_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/util/concurrent/BrokenBarrierException".to_string(),
    }
}
fn primitive_wrapper_type_descriptor(class_name: &str) -> Option<&'static str> {
    match class_name {
        "java/lang/Boolean" => Some("Z"),
        "java/lang/Byte" => Some("B"),
        "java/lang/Character" => Some("C"),
        "java/lang/Double" => Some("D"),
        "java/lang/Float" => Some("F"),
        "java/lang/Integer" => Some("I"),
        "java/lang/Long" => Some("J"),
        "java/lang/Short" => Some("S"),
        "java/lang/Void" => Some("V"),
        _ => None,
    }
}
fn internal_name_to_binary_name(name: &str) -> String {
    match name {
        "B" => "byte".to_string(),
        "C" => "char".to_string(),
        "D" => "double".to_string(),
        "F" => "float".to_string(),
        "I" => "int".to_string(),
        "J" => "long".to_string(),
        "S" => "short".to_string(),
        "Z" => "boolean".to_string(),
        "V" => "void".to_string(),
        _ => name.replace('/', "."),
    }
}
/// Box a primitive `Slot` into a heap object so it can be stored as `Object` in collections.
/// `Slot::Reference` and `Slot::Long` pad pass through unchanged.
/// `Slot::Int` → `java/lang/Integer`, `Slot::Long` → `java/lang/Long`,
/// `Slot::Double` → `java/lang/Double`, `Slot::Float` → `java/lang/Float`.
fn box_primitive_slot(slot: Slot, heap: &mut duke_gc::Heap) -> Slot {
    match slot {
        Slot::Int(v) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Int(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Long(v) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Long(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Double(v) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Double(v);
            }
            Slot::Reference(Some(r))
        }
        Slot::Float(v) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            if let Ok(obj) = heap.get_mut(r) {
                obj.fields[0] = Slot::Float(v);
            }
            Slot::Reference(Some(r))
        }
        other => other,
    }
}
fn box_reflection_return_value(
    heap: &mut duke_gc::Heap,
    return_type: char,
    result: Option<Slot>,
) -> Result<Slot> {
    match return_type {
        'V' => Ok(Slot::Reference(None)),
        'L' | '[' => Ok(result.unwrap_or(Slot::Reference(None))),
        'B' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "byte result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Byte".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'C' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "char result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Character".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'D' => {
            let Some(Slot::Double(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "double result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'F' => {
            let Some(Slot::Float(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "float result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Float(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'I' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "int result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'J' => {
            let Some(Slot::Long(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "long result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'S' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "short result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Short".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'Z' => {
            let Some(Slot::Int(value)) = result else {
                return Err(Error::TypeMismatch {
                    expected: "boolean result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        _ => {
            Err(Error::Unimplemented {
                mnemonic: "reflection primitive boxing",
            })
        }
    }
}
fn filtered_base64_input(input: &[u8], variant: Base64Variant) -> Vec<u8> {
    input
        .iter()
        .copied()
        .filter(|byte| {
            variant != Base64Variant::Mime
                || !matches!(byte, b'\r' | b'\n' | b' ' | b'\t')
        })
        .collect()
}
pub(crate) fn native_code_source_get_location(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
}
/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
pub(crate) fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let port = extract_int_arg(args, 1)?;
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    }
    obj.fields[0] = Slot::Int(server_id);
    obj.fields[1] = Slot::Int(actual_port);
    Ok(None)
}
/// Native: `ServerSocket.accept()` — blocks until a client connects, returns a Socket.
pub(crate) fn native_server_socket_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let (reader_id, writer_id) = heap.accept_connection(server_fd)?;
    let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
    heap.get_mut(socket_ref)?.fields[0] = Slot::Int(reader_id);
    heap.get_mut(socket_ref)?.fields[1] = Slot::Int(writer_id);
    Ok(Some(Slot::Reference(Some(socket_ref))))
}
/// Native: `ServerSocket.getLocalPort()` — returns the bound port.
pub(crate) fn native_server_socket_get_local_port(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => {
            Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            })
        }
    }
}
/// Native: `ServerSocket.close()` — closes the OS listener and zeros the fd field.
pub(crate) fn native_server_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None),
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}
fn copy_group_name(chars: &[char], start: usize, out: &mut String) -> Option<usize> {
    let mut idx = start;
    while idx < chars.len() {
        let ch = chars[idx];
        out.push(ch);
        idx += 1;
        if ch == '>' {
            return Some(idx);
        }
    }
    None
}
fn monotonic_nano_time_now() -> i64 {
    let origin = NANO_TIME_ORIGIN.get_or_init(std::time::Instant::now);
    i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
}
fn init_string_from_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    bytes: &[u8],
    charset: StandardCharset,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let decoded = decode_string_with_charset(bytes, charset);
    heap.get_mut(this_ref)?.string_value = Some(decoded);
    Ok(None)
}
fn init_future(heap: &mut duke_gc::Heap, result: Slot, task: Slot) -> Result<u64> {
    let future_ref = heap.allocate("duke/util/concurrent/DukeFuture".to_string(), 5);
    {
        let future = heap.get_mut(future_ref)?;
        future.fields[FUTURE_STATE_FIELD] = Slot::Int(FUTURE_PENDING);
        future.fields[FUTURE_RESULT_FIELD] = result;
        future.fields[FUTURE_EXCEPTION_FIELD] = Slot::Reference(None);
        future.fields[FUTURE_WAIT_DEADLINE_FIELD] = Slot::Long(0);
        future.fields[FUTURE_TASK_FIELD] = task;
    }
    heap.remember_reference_write(future_ref, result);
    heap.remember_reference_write(future_ref, task);
    Ok(future_ref)
}
/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L...;` / `[...]`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse();
    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref) && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}
fn init_properties_with_defaults(
    heap: &mut duke_gc::Heap,
    props_ref: u64,
    defaults: Slot,
) -> Result<()> {
    ensure_properties_layout(heap, props_ref)?;
    let props = heap.get_mut(props_ref)?;
    props.fields[PROPERTIES_SIZE_FIELD] = Slot::Int(0);
    props.fields[PROPERTIES_DEFAULTS_FIELD] = defaults;
    props.fields.truncate(PROPERTIES_ENTRIES_START);
    Ok(())
}
#[allow(clippy::cast_possible_wrap)]
/// Convert a heap object to its Java display string.
///
/// Checks `string_value` first (handles String/StringBuilder).
/// For boxed primitives, extracts the stored value from `fields[0]`.
/// Falls back to `class_name@hex_ref` for opaque objects.
fn heap_object_to_string(obj: &duke_gc::HeapObject, obj_ref: u64) -> String {
    if let Some(s) = &obj.string_value {
        return s.clone();
    }
    match obj.class_name.as_str() {
        "java/lang/Integer" => {
            if let Some(Slot::Int(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Long" => {
            if let Some(Slot::Long(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Double" => {
            if let Some(Slot::Double(v)) = obj.fields.first() {
                return format_java_double(*v);
            }
        }
        "java/lang/Float" => {
            if let Some(Slot::Float(v)) = obj.fields.first() {
                return format_java_float(*v);
            }
        }
        "java/lang/Boolean" => {
            return match obj.fields.first() {
                Some(Slot::Int(v)) if *v != 0 => "true".to_string(),
                _ => "false".to_string(),
            };
        }
        "java/lang/Character" => {
            if let Some(Slot::Int(v)) = obj.fields.first()
                && let Some(c) = char::from_u32((*v).cast_unsigned())
            {
                return c.to_string();
            }
        }
        _ => {}
    }
    format!("{}@{:x}", obj.class_name, obj_ref)
}
fn open_boot_archive_reader(path: &std::path::Path) -> Result<duke_loader::ZipReader> {
    duke_loader::ZipReader::open(path)
        .map_err(|err| match err {
            duke_loader::Error::Io { .. } => {
                Error::JavaException {
                    class_name: "java/io/IOException".to_string(),
                }
            }
            _ => {
                Error::JavaException {
                    class_name: "java/util/zip/ZipException".to_string(),
                }
            }
        })
}
/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// Sum a class's instance fields across its full superclass chain.
fn total_instance_field_count(registry: &ClassRegistry, class_name: &str) -> usize {
    let mut count = registry.get(class_name).map_or(0, |c| c.instance_field_count);
    let mut sc = registry.get(class_name).ok().and_then(|c| c.super_class.clone());
    while let Some(ref s) = sc {
        match registry.get(s) {
            Ok(sctx) => {
                count += sctx.instance_field_count;
                sc = sctx.super_class.clone();
            }
            Err(_) => break,
        }
    }
    count
}
fn hex_value(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => Some(u32::from(ch) - u32::from('0')),
        'a'..='f' => Some(u32::from(ch) - u32::from('a') + 10),
        'A'..='F' => Some(u32::from(ch) - u32::from('A') + 10),
        _ => None,
    }
}
fn descriptor_class_key_from_source(
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> Result<String> {
    match descriptor.as_bytes().first().copied() {
        Some(
            b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'V',
        ) if descriptor.len() == 1 => Ok(descriptor.to_string()),
        Some(b'[') => Ok(descriptor.to_string()),
        Some(b'L') if descriptor.ends_with(';') => {
            ops.class_key_from_source(&descriptor[1..descriptor.len() - 1], source_class)
        }
        _ => {
            Err(Error::TypeMismatch {
                expected: "type descriptor",
                got: "other",
            })
        }
    }
}
fn descriptor_class_slot_from_source(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    source_class: Option<&str>,
) -> Result<Slot> {
    let class_key = descriptor_class_key_from_source(ops, descriptor, source_class)?;
    let class_ref = allocate_class_object(heap, &class_key)?;
    Ok(Slot::Reference(Some(class_ref)))
}
fn descriptor_parameter_part(descriptor: &str) -> &str {
    descriptor.find(')').map_or(descriptor, |idx| &descriptor[..=idx])
}
fn descriptor_return_type(descriptor: &str) -> char {
    descriptor.split_once(')').and_then(|(_, ret)| ret.chars().next()).unwrap_or('V')
}
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (
                decode_pct_hex(bytes[i + 1]),
                decode_pct_hex(bytes[i + 2]),
            )
        {
            out.push(char::from((hi << 4) | lo));
            i += 3;
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}
pub(crate) fn native_executors_new_fixed_thread_pool(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let count = extract_int_arg(args, 0)?;
    if count <= 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IllegalArgumentException".to_string(),
        });
    }
    Ok(Some(allocate_executor(heap, usize::try_from(count).unwrap_or(1))?))
}
pub(crate) fn native_executors_new_single_thread_executor(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(allocate_executor(heap, 1)?))
}
pub(crate) fn native_executors_new_cached_thread_pool(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(allocate_executor(heap, 64)?))
}
/// Convert a Slot to its string representation (like Java's String.valueOf).
/// `type_hint` is the JVM type descriptor char: 'Z' for boolean, 'I' for int, etc.
fn stringify_slot(
    slot: &Slot,
    type_hint: char,
    heap: &duke_gc::Heap,
    out: &mut String,
) -> Result<()> {
    match slot {
        Slot::Int(v) => {
            if type_hint == 'Z' {
                out.push_str(if *v != 0 { "true" } else { "false" });
            } else if type_hint == 'C' {
                if let Some(ch) = char::from_u32((*v).cast_unsigned()) {
                    out.push(ch);
                } else {
                    out.push('?');
                }
            } else {
                out.push_str(&v.to_string());
            }
        }
        Slot::Long(v) => out.push_str(&v.to_string()),
        Slot::Float(v) => out.push_str(&format_java_float(*v)),
        Slot::Double(v) => out.push_str(&format_java_double(*v)),
        Slot::Reference(None) => out.push_str("null"),
        Slot::Reference(Some(r)) => {
            out.push_str(&heap_object_to_string(heap.get(*r)?, *r));
        }
        Slot::ReturnAddress(v) => out.push_str(&v.to_string()),
    }
    Ok(())
}
fn slot_string(heap: &duke_gc::Heap, slot: Slot) -> Result<Option<String>> {
    match slot {
        Slot::Reference(Some(r)) => Ok(heap.get(r)?.string_value.clone()),
        _ => Ok(None),
    }
}
/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> Result<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Int (char)",
                got: "other",
            })
        }
    }
}
/// Helper: read a string from a slot (returns "" for null).
fn slot_to_string(slot: Slot, heap: &duke_gc::Heap) -> String {
    match slot {
        Slot::Reference(Some(r)) => {
            heap.get(r).ok().and_then(|o| o.string_value.clone()).unwrap_or_default()
        }
        _ => String::new(),
    }
}
fn slot_is_java_string(heap: &duke_gc::Heap, slot: Slot) -> bool {
    let Slot::Reference(Some(string_ref)) = slot else {
        return false;
    };
    heap.get(string_ref)
        .is_ok_and(|obj| {
            obj.class_name == "java/lang/String" && obj.string_value.is_some()
        })
}
fn unsupported_charset_error(name: &str) -> Error {
    push_pending_java_exception_message(
        "java/nio/charset/UnsupportedCharsetException",
        name.to_string(),
    );
    Error::JavaException {
        class_name: "java/nio/charset/UnsupportedCharsetException".to_string(),
    }
}
fn unsupported_encoding_error(name: &str) -> Error {
    push_pending_java_exception_message(
        "java/io/UnsupportedEncodingException",
        name.to_string(),
    );
    Error::JavaException {
        class_name: "java/io/UnsupportedEncodingException".to_string(),
    }
}
fn unsupported_operation_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    }
}
fn annotations_for_reflected_method(
    heap: &duke_gc::Heap,
    method_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(
        ops
            .inspect_class(&method.declaring_class_key)?
            .methods
            .into_iter()
            .find(|candidate| {
                let name_matches = candidate.name == method.method_name;
                let descriptor_matches = candidate.descriptor == method.descriptor;
                name_matches && descriptor_matches
            })
            .map_or_else(Vec::new, |method| method.annotations),
    )
}
fn annotations_for_reflected_field(
    heap: &duke_gc::Heap,
    field_ref: u64,
    ops: &mut dyn CallbackOps,
) -> Result<Vec<ReflectedAnnotation>> {
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(
        ops
            .inspect_class(&field.declaring_class_key)?
            .fields
            .into_iter()
            .find(|candidate| {
                let name_matches = candidate.name == field.field_name;
                let descriptor_matches = candidate.descriptor == field.descriptor;
                name_matches && descriptor_matches
            })
            .map_or_else(Vec::new, |field| field.annotations),
    )
}
fn run_executor_state_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<Option<Slot>> {
    loop {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm { registry, heap, output, live_workers } = &mut *shared_guard;
        let outcome = execution::run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);
        match outcome {
            ExecutionOutcome::Returned(result) => return Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
            ExecutionOutcome::Yield => {
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    }
}
fn run_executor_task(
    task: duke_gc::ExecutorTask,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if future_state(&shared_guard.heap, task.future_ref)? == FUTURE_CANCELLED {
            return Ok(());
        }
        shared_guard
            .heap
            .write_field(
                task.future_ref,
                FUTURE_STATE_FIELD,
                Slot::Int(FUTURE_RUNNING),
            )?;
    }
    let invocation = {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm { registry, heap, output, .. } = &mut *shared_guard;
        let result = prepare_executor_invocation(
            registry,
            loader.as_ref(),
            heap,
            output,
            task,
        );
        drop(shared_guard);
        result
    };
    let invocation = match invocation {
        Ok(invocation) => invocation,
        Err(Error::JavaException { class_name }) => {
            let mut shared_guard = shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let CompletionVm { registry, heap, .. } = &mut *shared_guard;
            let result = store_future_failure(
                registry,
                loader.as_ref(),
                heap,
                task.future_ref,
                &class_name,
            );
            drop(shared_guard);
            result?;
            return Ok(());
        }
        Err(err) => return Err(err),
    };
    let impl_desc = invocation.impl_desc.clone();
    let sam_desc = invocation.sam_desc.clone();
    match run_executor_state_to_completion(invocation.state, shared, runtime, loader) {
        Ok(result) => {
            let mut shared_guard = shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let result = store_future_success(
                &mut shared_guard.heap,
                task,
                result,
                impl_desc.as_deref(),
                sam_desc.as_deref(),
            );
            drop(shared_guard);
            result
        }
        Err(Error::JavaException { class_name }) => {
            let mut shared_guard = shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let CompletionVm { registry, heap, .. } = &mut *shared_guard;
            let result = store_future_failure(
                registry,
                loader.as_ref(),
                heap,
                task.future_ref,
                &class_name,
            );
            drop(shared_guard);
            result?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}
fn run_executor_worker(
    executor: &duke_gc::ExecutorShared,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    loop {
        let task = {
            let mut guard = executor
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                if let Some(task) = guard.queue.pop_front() {
                    guard.active = guard.active.saturating_add(1);
                    break task;
                }
                if guard.shutdown {
                    guard.workers = guard.workers.saturating_sub(1);
                    guard.refresh_terminated();
                    drop(guard);
                    executor.available.notify_all();
                    return Ok(());
                }
                guard = executor
                    .available
                    .wait(guard)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        let result = run_executor_task(task, shared, runtime, loader);
        {
            let mut guard = executor
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.active = guard.active.saturating_sub(1);
            guard.refresh_terminated();
            drop(guard);
            executor.available.notify_all();
        }
        result?;
    }
}
fn run_thread_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    loop {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm { registry, heap, output, live_workers } = &mut *shared_guard;
        let outcome = execution::run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);
        match outcome {
            ExecutionOutcome::Returned(_) => return Ok(()),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
            ExecutionOutcome::Yield => {
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    }
}
fn invalid_base64_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}
fn reset_matcher_fields(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.fields[MATCHER_APPEND_POS_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}
/// Compare two `TreeMap` keys by natural ordering, supporting String, Integer, Long, and Double keys.
fn compare_treemap_keys(a: Slot, b: Slot, heap: &duke_gc::Heap) -> std::cmp::Ordering {
    let key_ord = |s: Slot| -> Option<KeyOrd> {
        if let Slot::Reference(Some(r)) = s && let Ok(obj) = heap.get(r) {
            if let Some(sv) = &obj.string_value {
                return Some(KeyOrd::Str(sv.clone()));
            }
            match obj.class_name.as_str() {
                "java/lang/Integer" | "java/lang/Short" | "java/lang/Byte" => {
                    if let Some(Slot::Int(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(i64::from(*n)));
                    }
                }
                "java/lang/Long" => {
                    if let Some(Slot::Long(n)) = obj.fields.first() {
                        return Some(KeyOrd::Int(*n));
                    }
                }
                "java/lang/Double" | "java/lang/Float" => {
                    if let Some(Slot::Double(n)) = obj.fields.first() {
                        return Some(KeyOrd::Flt(*n));
                    }
                }
                _ => {}
            }
        }
        None
    };
    match (key_ord(a), key_ord(b)) {
        (Some(KeyOrd::Int(x)), Some(KeyOrd::Int(y))) => x.cmp(&y),
        (Some(KeyOrd::Flt(x)), Some(KeyOrd::Flt(y))) => x.total_cmp(&y),
        (Some(KeyOrd::Str(x)), Some(KeyOrd::Str(y))) => x.cmp(&y),
        _ => std::cmp::Ordering::Equal,
    }
}
fn compare_slots_natural(
    a: Slot,
    b: Slot,
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    ops: &mut dyn CallbackOps,
) -> Result<i32> {
    match (a, b) {
        (Slot::Reference(Some(ra)), Slot::Reference(Some(_rb))) => {
            let a_class = heap.get(ra)?.class_name.clone();
            let result = ops
                .invoke(
                    heap,
                    out,
                    &a_class,
                    "compareTo",
                    "(Ljava/lang/Object;)I",
                    vec![a, b],
                )?
                .unwrap_or(Slot::Int(0));
            Ok(
                match result {
                    Slot::Int(n) => n,
                    _ => 0,
                },
            )
        }
        (Slot::Int(a), Slot::Int(b)) => Ok(a.cmp(&b) as i32),
        _ => Ok(0),
    }
}
/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
#[allow(clippy::used_underscore_binding, clippy::cast_possible_truncation)]
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,
) -> Result<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    registry.mark_initialized(class_name);
    initialize_primitive_wrapper_type_field(registry, heap, class_name)?;
    let has_clinit = registry
        .get(class_name)
        .is_ok_and(|ctx| {
            ctx.methods.iter().any(|m| m.name == "<clinit>" && m.descriptor == "()V")
        });
    if has_clinit {
        let class_loader = registry.class_loader(class_name).cloned();
        let init_loader = class_loader.as_deref().map_or(loader, |v| v);
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            init_loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry
            .telemetry
            .class_init_dag
            .record(
                class_name,
                _triggered_by,
                _clinit_start.elapsed().as_nanos() as u64,
            );
    }
    Ok(())
}
fn ensure_properties_layout(heap: &mut duke_gc::Heap, props_ref: u64) -> Result<()> {
    let props = heap.get_mut(props_ref)?;
    if props.fields.len() < PROPERTIES_ENTRIES_START {
        props.fields.resize(PROPERTIES_ENTRIES_START, Slot::Reference(None));
    }
    Ok(())
}
/// Native: `StringBuilder.setCharAt(int, char) -> void`
///
/// **Bolt Optimization:**
/// Eliminates a `Vec<char>` intermediate allocation by utilizing `char_indices` to map
/// character indexes to byte offsets, allowing direct, in-place `replace_range` mutations on the `String`.
pub(crate) fn native_stringbuilder_set_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(usize::MAX);
    let ch = match args.get(2) {
        Some(Slot::Int(v)) => {
            char::from_u32(u32::from_ne_bytes(v.to_ne_bytes())).unwrap_or('\0')
        }
        _ => '\0',
    };
    let buf = heap.get_mut(this_ref)?.string_value.get_or_insert_with(String::new);
    if let Some((byte_offset, old_ch)) = buf.char_indices().nth(idx) {
        let mut b = [0; 4];
        buf.replace_range(
            byte_offset..byte_offset + old_ch.len_utf8(),
            ch.encode_utf8(&mut b),
        );
    }
    Ok(None)
}
fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> Result<()> {
    loop {
        let handle = {
            let mut runtime = runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }
            let is_self_join = runtime
                .handles
                .get(&thread_id)
                .is_some_and(|h| h.thread().id() == std::thread::current().id());
            if is_self_join {
                drop(runtime);
                loop {
                    std::thread::park();
                }
            }
            runtime.handles.remove(&thread_id)
        };
        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }
        std::thread::yield_now();
    }
}
fn canonical_digest_algorithm(algorithm: &str) -> Option<&'static str> {
    let compact: String = algorithm
        .bytes()
        .filter(|byte| !matches!(byte, b'-' | b'_' | b' ' | b'\t' | b'\r' | b'\n'))
        .map(|byte| char::from(byte.to_ascii_uppercase()))
        .collect();
    match compact.as_str() {
        "SHA256" => Some("SHA-256"),
        "SHA1" => Some("SHA-1"),
        "MD5" => Some("MD5"),
        _ => None,
    }
}
fn method_return_descriptor(descriptor: &str) -> &str {
    descriptor.split_once(')').map_or("V", |(_, ret)| ret)
}
fn handle_thread_action(
    action: NativeThreadAction,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> Result<()> {
    match action {
        NativeThreadAction::Start { thread_ref } => {
            spawn_java_thread(shared, runtime, loader, thread_ref)
        }
        NativeThreadAction::Sleep(duration) => {
            std::thread::sleep(duration);
            Ok(())
        }
        NativeThreadAction::Join { thread_id } => join_java_thread(runtime, thread_id),
        NativeThreadAction::Retry => {
            std::thread::sleep(std::time::Duration::from_micros(1));
            Ok(())
        }
        NativeThreadAction::ExecutorSubmit {
            executor_ref,
            future_ref,
            task_ref,
            kind,
        } => {
            enqueue_executor_task(
                shared,
                runtime,
                loader,
                executor_ref,
                future_ref,
                task_ref,
                kind,
            )
        }
    }
}
pub(crate) fn native_boolean_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(i32::from(val != 0));
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_boolean_booleanvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_boolean_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bool_val = |s: &Slot| -> Result<bool> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Int(n)) => Ok(*n != 0),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => bool_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = bool_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}
/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
pub(crate) fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
fn saturating_mul_i64(lhs: i64, rhs: i64) -> i64 {
    let value = i128::from(lhs).saturating_mul(i128::from(rhs));
    let clamped = value.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    match i64::try_from(clamped) {
        Ok(value) => value,
        Err(_) if clamped < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}
/// Native: mutation ops on `UnmodifiableList` throw `UnsupportedOperationException`.
pub(crate) fn native_unmodifiable_list_mutation(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Err(Error::JavaException {
        class_name: "java/lang/UnsupportedOperationException".to_string(),
    })
}
fn first_reference_field(heap: &duke_gc::Heap, obj_ref: u64) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(*r)),
        Some(Slot::Reference(None)) => Ok(None),
        _ => Err(Error::NullPointerException),
    }
}
/// Return the first character of the return type portion of a method descriptor.
fn desc_return_char(desc: &str) -> Option<char> {
    desc.split_once(')').and_then(|(_, ret)| ret.chars().next())
}
#[cfg(feature = "telemetry")]
#[allow(clippy::too_many_lines)]
const fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => {
            "iconst_n"
        }
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => {
            "iload"
        }
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => {
            "lload"
        }
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => {
            "fload"
        }
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => {
            "dload"
        }
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => {
            "aload"
        }
        I::Istore(_)
        | I::Istore0
        | I::Istore1
        | I::Istore2
        | I::Istore3
        | I::IstoreW(_) => "istore",
        I::Lstore(_)
        | I::Lstore0
        | I::Lstore1
        | I::Lstore2
        | I::Lstore3
        | I::LstoreW(_) => "lstore",
        I::Fstore(_)
        | I::Fstore0
        | I::Fstore1
        | I::Fstore2
        | I::Fstore3
        | I::FstoreW(_) => "fstore",
        I::Dstore(_)
        | I::Dstore0
        | I::Dstore1
        | I::Dstore2
        | I::Dstore3
        | I::DstoreW(_) => "dstore",
        I::Astore(_)
        | I::Astore0
        | I::Astore1
        | I::Astore2
        | I::Astore3
        | I::AstoreW(_) => "astore",
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
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => {
            "if_<cond>"
        }
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
fn take_current_host_thread_interrupted() -> bool {
    interrupted_host_threads()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&current_host_thread_id())
}
fn take_uncaught_java_exception_ref(class_name: &str) -> Option<u64> {
    let mut refs = uncaught_java_exception_refs()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let thread_id = std::thread::current().id();
    let queue = refs.get_mut(&thread_id)?;
    let position = queue
        .iter()
        .position(|(queued_class, _)| queued_class == class_name)
        .unwrap_or(0);
    let (_, exception_ref) = queue.remove(position)?;
    if queue.is_empty() {
        refs.remove(&thread_id);
    }
    drop(refs);
    Some(exception_ref)
}
fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> Result<()> {
    let mut first_error = None;
    loop {
        let handles = {
            let mut runtime = runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if runtime.handles.is_empty() && runtime.executor_handles.is_empty() {
                return first_error.unwrap_or(Ok(()));
            }
            let mut handles = Vec::with_capacity(
                runtime.handles.len() + runtime.executor_handles.len(),
            );
            for (_, handle) in runtime.handles.drain() {
                handles.push(handle);
            }
            for (_, handle) in runtime.executor_handles.drain() {
                handles.push(handle);
            }
            drop(runtime);
            handles
        };
        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    if let Err(e) = result {
                        first_error.get_or_insert(Err(e));
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_false_boolean(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(0)))
}
fn unregister_java_host_thread(host_key: i32) {
    let removed_host_thread = java_thread_hosts()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&host_key);
    if let Some(host_thread_id) = removed_host_thread {
        interrupted_host_threads()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&host_thread_id);
    }
}
fn illegal_monitor_state_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalMonitorStateException".to_string(),
    }
}
fn illegal_argument_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    }
}
const fn request_native_retry(control: &mut NativeControl) {
    control.request(NativeThreadAction::Retry);
}
pub(crate) fn native_printstream_init_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    set_object_field(heap, this_ref, 0, target)?;
    Ok(None)
}
fn capture_stack_trace_snapshot(
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
    bci: usize,
    call_stack: &[CallFrame],
) -> Vec<NativeStackFrame> {
    let mut frames = Vec::with_capacity(call_stack.len() + 1);
    if let Some(frame) = stack_frame_for_method(
        registry,
        current_class,
        method_idx,
        bci,
    ) {
        frames.push(frame);
    }
    for caller in call_stack.iter().rev() {
        let caller_bci = registry
            .get(&caller.class_name)
            .ok()
            .and_then(|ctx| ctx.methods.get(caller.method_idx))
            .and_then(|method| {
                caller
                    .resume_idx
                    .checked_sub(1)
                    .and_then(|idx| method.instructions.get(idx))
                    .map(|(pc, _)| *pc)
            })
            .unwrap_or(0);
        if let Some(frame) = stack_frame_for_method(
            registry,
            &caller.class_name,
            caller.method_idx,
            caller_bci,
        ) {
            frames.push(frame);
        }
    }
    frames
}
fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> Result<std::path::PathBuf> {
    let path_ref = extract_ref_arg(args, idx)?;
    let path = heap
        .get(path_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    Ok(std::path::PathBuf::from(path))
}
fn path_to_file_url(path: &std::path::Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
            normalized = format!("//{stripped}");
        } else if let Some(stripped) = normalized.strip_prefix("//?/") {
            normalized = stripped.to_string();
        }
    }
    if cfg!(windows) && !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    format!("file://{normalized}")
}
pub(crate) fn native_path_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let uri_ref = extract_ref_arg(args, 0)?;
    let uri = string_backed_object_value(heap, uri_ref)?;
    let path = file_url_to_path(&uri)?;
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().to_string(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}
pub(crate) fn native_path_to_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .ok_or(Error::NullPointerException)?;
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(file_ref)?.fields[0] = path_slot;
    Ok(Some(Slot::Reference(Some(file_ref))))
}
/// Native: `JarFile.<init>(File)` — open and index a JAR archive from a File object.
pub(crate) fn native_jar_file_init_from_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_ref = extract_ref_arg(args, 1)?;
    let path = file_path_from_ref(file_ref, heap)?;
    let fd = zip_open(&path)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}
pub(crate) fn native_jar_file_init_with_mode_and_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let forwarded_args = match args {
        [this_slot, file_slot, ..] => [*this_slot, *file_slot],
        _ => {
            return Err(Error::TypeMismatch {
                expected: "this,file",
                got: "other",
            });
        }
    };
    native_jar_file_init_from_file(&forwarded_args, heap, out, control)
}
pub(crate) fn native_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let Ok(manifest_bytes) = zip_read_entry(fd, "META-INF/MANIFEST.MF") else {
        return Ok(Some(Slot::Reference(None)));
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}
fn expand_args_for_desc(args: &[Slot], descriptor: &str) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    let wide_count = param_types.iter().filter(|&&c| c == 'J' || c == 'D').count();
    if wide_count == 0 || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + wide_count);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        result.push(*slot);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0));
        }
    }
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}
fn expand_ascii_case_insensitive(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut idx = 0;
    let mut escaped = false;
    let mut in_class = false;
    while idx < chars.len() {
        let ch = chars[idx];
        if escaped {
            out.push(ch);
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            out.push(ch);
            escaped = true;
            idx += 1;
            continue;
        }
        if !in_class && ch == '(' && chars.get(idx + 1) == Some(&'?') {
            if chars.get(idx + 2) == Some(&'P') && chars.get(idx + 3) == Some(&'<') {
                out.push_str("(?P<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 4, &mut out) {
                    idx = next_idx;
                    continue;
                }
            } else if chars.get(idx + 2) == Some(&'<')
                && !matches!(chars.get(idx + 3), Some('=' | '!') | None)
            {
                out.push_str("(?<");
                if let Some(next_idx) = copy_group_name(&chars, idx + 3, &mut out) {
                    idx = next_idx;
                    continue;
                }
            }
        }
        match ch {
            '[' => {
                in_class = true;
                out.push(ch);
            }
            ']' if in_class => {
                in_class = false;
                out.push(ch);
            }
            _ if !in_class && ch.is_ascii_alphabetic() => {
                out.push('[');
                out.push(ch.to_ascii_lowercase());
                out.push(ch.to_ascii_uppercase());
                out.push(']');
            }
            _ => out.push(ch),
        }
        idx += 1;
    }
    out
}
fn encode_string_with_charset(value: &str, charset: StandardCharset) -> Vec<u8> {
    match charset {
        StandardCharset::Utf8 => value.as_bytes().to_vec(),
        StandardCharset::UsAscii => {
            value
                .chars()
                .map(|ch| { if ch <= '\u{7f}' { ch as u8 } else { b'?' } })
                .collect()
        }
        StandardCharset::Iso88591 => {
            value
                .chars()
                .map(|ch| { if u32::from(ch) <= 0xff { ch as u8 } else { b'?' } })
                .collect()
        }
        StandardCharset::Utf16 => {
            let mut bytes = Vec::with_capacity(2 + value.len().saturating_mul(2));
            bytes.extend_from_slice(&[0xfe, 0xff]);
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
        StandardCharset::Utf16Be => {
            let mut bytes = Vec::with_capacity(value.len().saturating_mul(2));
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
        StandardCharset::Utf16Le => {
            let mut bytes = Vec::with_capacity(value.len().saturating_mul(2));
            for unit in value.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            bytes
        }
    }
}
fn encode_base64(input: &[u8], variant: Base64Variant) -> String {
    let alphabet = variant.alphabet();
    let mut out = Vec::with_capacity(input.len().div_ceil(3) * 4);
    let mut line_len = 0_usize;
    for chunk in input.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (first << 16) | (second << 8) | third;
        let encoded = [
            alphabet[((triple >> 18) & 0x3f) as usize],
            alphabet[((triple >> 12) & 0x3f) as usize],
            if chunk.len() > 1 {
                alphabet[((triple >> 6) & 0x3f) as usize]
            } else {
                b'='
            },
            if chunk.len() > 2 { alphabet[(triple & 0x3f) as usize] } else { b'=' },
        ];
        for byte in encoded {
            if variant == Base64Variant::Mime && line_len == 76 {
                out.extend_from_slice(b"\r\n");
                line_len = 0;
            }
            out.push(byte);
            line_len += 1;
        }
    }
    out.into_iter().map(char::from).collect()
}
fn require_base64_value(byte: u8, variant: Base64Variant) -> Result<u8> {
    base64_decode_value(byte, variant).ok_or_else(invalid_base64_error)
}
fn require_digest_algorithm(algorithm: &str) -> Result<&'static str> {
    canonical_digest_algorithm(algorithm)
        .ok_or_else(|| Error::JavaException {
            class_name: "java/security/NoSuchAlgorithmException".to_string(),
        })
}
const fn require_chm_non_null(slot: Slot) -> Result<Slot> {
    if matches!(slot, Slot::Reference(None)) {
        Err(Error::NullPointerException)
    } else {
        Ok(slot)
    }
}
fn collection_elements_from_ref(
    heap: &duke_gc::Heap,
    collection_ref: u64,
) -> Result<Vec<Slot>> {
    let collection = heap.get(collection_ref)?;
    match collection.class_name.as_str() {
        "java/util/ArrayList" | "java/util/HashSet" => {
            let size = match collection.fields.first() {
                Some(Slot::Int(size)) if *size >= 0 => {
                    usize::try_from(*size).unwrap_or(0)
                }
                _ => 0,
            };
            Ok(collection.fields.iter().skip(1).take(size).copied().collect())
        }
        _ => {
            Err(Error::TypeMismatch {
                expected: "java/util/Collection",
                got: "other",
            })
        }
    }
}
/// Native: `Collection.toArray()` — copies Duke-backed collection elements into `Object[]`.
pub(crate) fn native_collection_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let array_ref = allocate_reference_array_from_slots(
        heap,
        "[Ljava/lang/Object;",
        &elements,
    )?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}
/// Native: `Collection.toArray(Object[])` — preserves the requested array type.
pub(crate) fn native_collection_to_array_with_seed_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed_array_ref = extract_ref_arg(args, 1)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let seed_class_name = heap.get(seed_array_ref)?.class_name.clone();
    let seed_len = heap.get(seed_array_ref)?.fields.len();
    if seed_len < elements.len() {
        let new_array_ref = allocate_reference_array_from_slots(
            heap,
            &seed_class_name,
            &elements,
        )?;
        return Ok(Some(Slot::Reference(Some(new_array_ref))));
    }
    let seed_array = heap.get_mut(seed_array_ref)?;
    for slot in &mut seed_array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        seed_array.fields[idx] = *element;
    }
    Ok(Some(Slot::Reference(Some(seed_array_ref))))
}
fn launched_class_loader_archive_path(
    registry: &ClassRegistry,
    heap: &duke_gc::Heap,
    loader_ref: u64,
) -> Result<Option<String>> {
    if heap.get(loader_ref)?.class_name
        != "org/springframework/boot/loader/launch/LaunchedClassLoader"
    {
        return Ok(None);
    }
    let root_archive_slot = field_slot_idx(
        registry,
        "org/springframework/boot/loader/launch/LaunchedClassLoader",
        "rootArchive",
    )?;
    let Some(archive_ref) = archive_ref_from_slot(heap, loader_ref, root_archive_slot)?
    else {
        return Ok(None);
    };
    boot_archive_path_from_ref(registry, heap, archive_ref)
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_zero_long(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(0)))
}
fn timeout_nanos(timeout: i64, unit_ref: u64, heap: &duke_gc::Heap) -> Result<i64> {
    Ok(saturating_mul_i64(timeout.max(0), timeunit_nanos_per_unit(heap, unit_ref)?))
}
fn timeout_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/util/concurrent/TimeoutException".to_string(),
    }
}
fn qualify_reflected_class_info_ancestry(
    registry: &ClassRegistry,
    source_class: &str,
    mut info: ReflectedClassInfo,
) -> ReflectedClassInfo {
    info.super_class = info
        .super_class
        .map(|super_class| {
            registry.class_key_from_source(&super_class, Some(source_class))
        });
    info.interfaces = info
        .interfaces
        .into_iter()
        .map(|interface| registry.class_key_from_source(&interface, Some(source_class)))
        .collect();
    info
}
fn classpath_debug_label(loader_ref: Option<u64>) -> String {
    loader_ref
        .map_or_else(|| "bootstrap".to_string(), |loader| format!("loader:{loader}"))
}
fn interrupt_host_thread(host_thread_id: std::thread::ThreadId) {
    interrupted_host_threads()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(host_thread_id);
}
/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]
#[allow(clippy::too_many_lines)]
fn build_method_entries(cf: &duke_classfile::ClassFile) -> Vec<MethodEntry> {
    use duke_bytecode::decode;
    use duke_classfile::MethodAccessFlags;
    use duke_classfile::types::{AttributeData, CpEntry};
    let source_file = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::SourceFile { sourcefile_index } = &a.data {
                match cf.constant_pool.get(sourcefile_index.0 as usize) {
                    Some(Some(CpEntry::Utf8(s))) => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            }
        });
    cf.methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let is_native = m.access_flags.contains(MethodAccessFlags::NATIVE);
            let is_abstract = m.access_flags.contains(MethodAccessFlags::ABSTRACT);
            let code = m
                .attributes
                .iter()
                .find_map(|a| {
                    if let AttributeData::Code(c) = &a.data { Some(c) } else { None }
                });
            let Some(code) = code else {
                if is_native || is_abstract {
                    return Some(MethodEntry {
                        name,
                        descriptor,
                        is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                        is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                        is_native,
                        is_abstract,
                        instructions: std::sync::Arc::new([]),
                        max_stack: 0,
                        max_locals: 0,
                        exception_table: vec![],
                        pc_to_idx: std::sync::Arc::new(std::collections::HashMap::new()),
                        line_number_table: Vec::new(),
                        source_file: source_file.clone(),
                    });
                }
                return None;
            };
            let line_number_table = code
                .attributes
                .iter()
                .find_map(|attr| {
                    if let AttributeData::LineNumberTable(entries) = &attr.data {
                        Some(
                            entries
                                .iter()
                                .map(|entry| (entry.start_pc, entry.line_number))
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None
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
                                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
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
                is_public: m.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: m.access_flags.contains(MethodAccessFlags::STATIC),
                is_native,
                is_abstract,
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
                line_number_table,
                source_file: source_file.clone(),
            })
        })
        .collect()
}
fn build_field_entries(
    cf: &duke_classfile::ClassFile,
) -> (Vec<FieldEntry>, Vec<Slot>, usize) {
    use duke_classfile::FieldAccessFlags;
    use duke_classfile::types::CpEntry;
    let mut fields = Vec::with_capacity(cf.fields.len());
    let mut static_fields = Vec::new();
    let mut instance_count = 0usize;
    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_fields.push(default_slot_for_descriptor(&descriptor));
        } else {
            instance_count += 1;
        }
        fields
            .push(FieldEntry {
                name,
                descriptor,
                is_static,
            });
    }
    (fields, static_fields, instance_count)
}
/// Constructs a `ClassContext` from a parsed `ClassFile`.
///
/// The `ClassContext` serves as the runtime representation of a loaded class.
/// It bridges the raw structure provided by `duke_classfile` and the execution environment
/// required by the interpreter. It encapsulates resolved metadata (like the class name and superclass),
/// method representations (including code and handlers), fields (both static and instance), and
/// bootstrap methods required for dynamic invocation (`invokedynamic`).
///
/// # Arguments
///
/// * `cf` - A reference to the parsed `duke_classfile::ClassFile`.
///
/// # Examples
///
/// ```
/// # use duke_interpreter::build_class_context;
/// # use duke_classfile::{ClassFile, ClassAccessFlags};
/// # use duke_classfile::types::{CpIndex, CpEntry};
/// // A minimal class file representation of `java/lang/Object`.
/// let cf = ClassFile {
///     minor_version: 0,
///     major_version: 52,
///     constant_pool: vec![
///         // Index 0 is implicit in Java constant pools, but duke_classfile uses 0-indexed vec
///         // Let's create a minimal valid pool where index 0 is a Utf8 and index 1 is a Class.
///         Some(CpEntry::Utf8("java/lang/Object".to_string())),
///         Some(CpEntry::Class { name_index: CpIndex(0) }),
///     ],
///     access_flags: ClassAccessFlags::empty(),
///     this_class: CpIndex(1), // Points to the Class entry at index 1
///     super_class: CpIndex(0), // No superclass
///     interfaces: vec![],
///     fields: vec![],
///     methods: vec![],
///     attributes: vec![],
/// };
///
/// let context = build_class_context(&cf);
/// assert_eq!(context.class_name, "java/lang/Object"); // Name is correctly resolved
/// ```
#[must_use]
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_classfile::types::{AttributeData, CpEntry};
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf.constant_pool.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };
    let methods = build_method_entries(cf);
    let (fields, static_fields, instance_field_count) = build_field_entries(cf);
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();
    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields,
        instance_field_count,
        bootstrap_methods,
        load_source: ClassLoadSource::Classfile,
    }
}
fn build_reflection_invoke_args(
    heap: &duke_gc::Heap,
    target_slot: Slot,
    descriptor: &str,
    invoke_arg_slots: Vec<Slot>,
    is_static: bool,
) -> Result<Vec<Slot>> {
    let arg_types = parse_arg_types(descriptor);
    if arg_types.len() != invoke_arg_slots.len() {
        return Err(Error::TypeMismatch {
            expected: "matching reflective argument count",
            got: "different count",
        });
    }
    let mut invoke_args = Vec::with_capacity(arg_types.len() + usize::from(!is_static));
    if !is_static {
        match target_slot {
            Slot::Reference(Some(_)) => invoke_args.push(target_slot),
            _ => return Err(Error::NullPointerException),
        }
    }
    for (descriptor, arg) in arg_types.iter().copied().zip(invoke_arg_slots) {
        invoke_args.push(unbox_reflection_argument(heap, descriptor, arg)?);
        if matches!(descriptor, 'J' | 'D') {
            invoke_args.push(Slot::Int(0));
        }
    }
    Ok(invoke_args)
}
fn archive_path_from_slot(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot_idx: usize,
) -> Result<Option<String>> {
    let Some(file_ref) = archive_ref_from_slot(heap, archive_ref, slot_idx)? else {
        return Ok(None);
    };
    Ok(Some(file_path_from_ref(file_ref, heap)?.to_string_lossy().to_string()))
}
fn archive_ref_from_slot(
    heap: &duke_gc::Heap,
    obj_ref: u64,
    slot_idx: usize,
) -> Result<Option<u64>> {
    match heap.get(obj_ref)?.fields.get(slot_idx).copied() {
        Some(Slot::Reference(Some(r))) => Ok(Some(r)),
        Some(Slot::Reference(None)) | None => Ok(None),
        Some(_) => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
fn archive_file_ref_at(
    heap: &duke_gc::Heap,
    archive_ref: u64,
    slot: usize,
) -> Result<u64> {
    match heap.get(archive_ref)?.fields.get(slot).copied() {
        Some(Slot::Reference(Some(file_ref))) => Ok(file_ref),
        Some(Slot::Reference(None)) => Err(Error::NullPointerException),
        _ => {
            Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            })
        }
    }
}
fn spawn_executor_worker(
    executor: std::sync::Arc<duke_gc::ExecutorShared>,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) {
    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let result = run_executor_worker(
            &executor,
            &shared_clone,
            &runtime_clone,
            &loader_clone,
        );
        {
            let mut shared = shared_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        result
    });
    let mut runtime_guard = runtime
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let worker_id = runtime_guard.next_executor_worker_id;
    runtime_guard.next_executor_worker_id = runtime_guard
        .next_executor_worker_id
        .wrapping_add(1);
    runtime_guard.executor_handles.insert(worker_id, handle);
}
fn spawn_java_thread(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    thread_ref: u64,
) -> Result<()> {
    {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let thread = shared_guard.heap.get_mut(thread_ref)?;
        let already_started = matches!(
            thread.fields.get(THREAD_ID_SLOT), Some(Slot::Int(thread_id)) if * thread_id
            >= 0
        );
        drop(shared_guard);
        if already_started {
            return Ok(());
        }
    }
    let host_key = next_thread_host_key();
    let thread_id = {
        let mut runtime = runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let thread_id = runtime.threads.allocate_thread_id();
        runtime.threads.register(threading::ThreadRecord::new(thread_ref, thread_id));
        thread_id
    };
    let entry = {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        {
            let thread = shared_guard.heap.get_mut(thread_ref)?;
            thread.fields[THREAD_ID_SLOT] = Slot::Int(thread_id);
            thread.fields[THREAD_HOST_KEY_SLOT] = Slot::Int(host_key);
        }
        let CompletionVm { registry, heap, live_workers, .. } = &mut *shared_guard;
        let entry = resolve_thread_entry(registry, loader.as_ref(), heap, thread_ref)?;
        if entry.is_some() {
            *live_workers += 1;
        }
        drop(shared_guard);
        entry
    };
    let Some((dispatch_class, method_idx, args)) = entry else {
        let _ = runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .threads
            .mark_finished(thread_id);
        return Ok(());
    };
    let state = {
        let shared = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        ExecutionState::new(&shared.registry, &dispatch_class, "run", method_idx, &args)?
    };
    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let host_thread_id = std::thread::current().id();
        register_java_host_thread(host_key, host_thread_id);
        let interrupted_before_start = {
            let shared = shared_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            matches!(
                shared.heap.get(thread_ref).ok().and_then(| thread | thread.fields
                .get(THREAD_INTERRUPTED_SLOT)).copied(), Some(Slot::Int(value)) if value
                != 0
            )
        };
        if interrupted_before_start {
            interrupt_host_thread(host_thread_id);
        }
        let result = run_thread_to_completion(
            state,
            &shared_clone,
            &runtime_clone,
            &loader_clone,
        );
        {
            let mut shared = shared_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        unregister_java_host_thread(host_key);
        result
    });
    runtime
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .handles
        .insert(thread_id, handle);
    Ok(())
}
fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> Result<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}
fn store_throwable_stack_trace_from_frames(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    frames: &[NativeStackFrame],
) -> Result<()> {
    let mut elements = Vec::with_capacity(frames.len());
    for frame in frames {
        let element_ref = allocate_stack_trace_element(
            heap,
            &frame.class_name,
            &frame.method_name,
            frame.file_name.as_deref(),
            frame.line_number,
        )?;
        elements.push(Slot::Reference(Some(element_ref)));
    }
    let array_ref = allocate_slot_array(heap, STACK_TRACE_ARRAY_CLASS, &elements)?;
    set_object_field(
        heap,
        throwable_ref,
        THROWABLE_STACK_TRACE_FIELD,
        Slot::Reference(Some(array_ref)),
    )
}
fn store_future_failure(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    future_ref: u64,
    class_name: &str,
) -> Result<()> {
    let cause_ref = if let Some(exception_ref) = take_uncaught_java_exception_ref(
        class_name,
    ) {
        exception_ref
    } else {
        materialize_java_exception_object(registry, loader, heap, class_name)?
    };
    heap.write_field(
        future_ref,
        FUTURE_EXCEPTION_FIELD,
        Slot::Reference(Some(cause_ref)),
    )?;
    heap.write_field(future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_FAILED))?;
    Ok(())
}
fn store_future_success(
    heap: &mut duke_gc::Heap,
    task: duke_gc::ExecutorTask,
    result: Option<Slot>,
    impl_desc: Option<&str>,
    sam_desc: Option<&str>,
) -> Result<()> {
    if future_state(heap, task.future_ref)? == FUTURE_CANCELLED {
        return Ok(());
    }
    let result = match task.kind {
        duke_gc::ExecutorTaskKind::Runnable => {
            extract_field_arg(heap, task.future_ref, FUTURE_RESULT_FIELD)?
        }
        duke_gc::ExecutorTaskKind::Callable => result.unwrap_or(Slot::Reference(None)),
    };
    let result = if let (Some(impl_desc), Some(sam_desc)) = (impl_desc, sam_desc) {
        autobox_if_needed(Some(result), impl_desc, sam_desc, heap)?
            .unwrap_or(Slot::Reference(None))
    } else {
        result
    };
    heap.write_field(task.future_ref, FUTURE_RESULT_FIELD, result)?;
    heap.remember_reference_write(task.future_ref, result);
    heap.write_field(task.future_ref, FUTURE_STATE_FIELD, Slot::Int(FUTURE_DONE))?;
    Ok(())
}
fn store_matcher_match(
    heap: &mut duke_gc::Heap,
    m_ref: u64,
    input: &str,
    start: usize,
    end: usize,
) -> Result<()> {
    let next_pos = next_find_pos(input, start, end);
    let start_i32 = i32::try_from(start).unwrap_or(i32::MAX);
    let end_i32 = i32::try_from(end).unwrap_or(i32::MAX);
    let next_i32 = i32::try_from(next_pos).unwrap_or(i32::MAX);
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_POS_FIELD] = Slot::Int(next_i32);
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(start_i32);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(end_i32);
    matcher.string_value = Some(input[start..end].to_string());
    Ok(())
}
fn inspect_reflected_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    class: &str,
) -> Result<ReflectedClassInfo> {
    let internal_name = registry.internal_name_for_class(class).to_string();
    match registry.resolve_loaded_class_key(class) {
        Ok(class_key) => {
            if let Some(class_loader) = registry.class_loader(&class_key)
                && let Some(info) = reflected_class_info_from_loader(
                    class_loader.as_ref(),
                    &internal_name,
                )
            {
                return Ok(
                    qualify_reflected_class_info_ancestry(registry, &class_key, info),
                );
            }
            if let Some(info) = reflected_class_info_from_loader(
                loader,
                &internal_name,
            ) {
                return Ok(
                    qualify_reflected_class_info_ancestry(registry, &class_key, info),
                );
            }
            let ctx = registry.get(&class_key)?;
            let methods = ctx
                .methods
                .iter()
                .map(|method| ReflectedMethodInfo {
                    name: method.name.clone(),
                    descriptor: method.descriptor.clone(),
                    is_public: method.is_public,
                    is_static: method.is_static,
                    annotations: Vec::new(),
                    annotation_default: None,
                })
                .collect();
            let fields = ctx
                .fields
                .iter()
                .map(|field| ReflectedFieldInfo {
                    name: field.name.clone(),
                    descriptor: field.descriptor.clone(),
                    is_public: true,
                    is_static: field.is_static,
                    annotations: Vec::new(),
                })
                .collect();
            return Ok(ReflectedClassInfo {
                internal_name: internal_name.clone(),
                binary_name: internal_name_to_binary_name(&internal_name),
                super_class: ctx.super_class.clone(),
                interfaces: ctx.interfaces.clone(),
                methods,
                fields,
                annotations: Vec::new(),
            });
        }
        Err(Error::ClassNotFound { .. }) => {}
        Err(err) => return Err(err),
    }
    if !registry.contains(class)
        && let Some(info) = reflected_class_info_from_loader(loader, &internal_name)
    {
        return Ok(info);
    }
    if !registry.contains(class) {
        registry.ensure_loaded(&internal_name, loader)?;
    }
    let class_key = registry.resolve_loaded_class_key(class)?;
    let ctx = registry.get(&class_key)?;
    let methods = ctx
        .methods
        .iter()
        .map(|method| ReflectedMethodInfo {
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            is_public: method.is_public,
            is_static: method.is_static,
            annotations: Vec::new(),
            annotation_default: None,
        })
        .collect();
    let fields = ctx
        .fields
        .iter()
        .map(|field| ReflectedFieldInfo {
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
            is_public: true,
            is_static: field.is_static,
            annotations: Vec::new(),
        })
        .collect();
    Ok(ReflectedClassInfo {
        internal_name: internal_name.clone(),
        binary_name: internal_name_to_binary_name(&internal_name),
        super_class: ctx.super_class.clone(),
        interfaces: ctx.interfaces.clone(),
        methods,
        fields,
        annotations: Vec::new(),
    })
}
fn split_properties_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    lines.push(current);
    lines
}
#[allow(clippy::too_many_arguments)]
fn prepare_execution_state(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> Result<ExecutionState> {
    let class_name = match registry.resolve_loaded_class_key(class_name) {
        Ok(class_key) => class_key,
        Err(Error::ClassNotFound { .. }) => {
            if !registry.ensure_loaded(class_name, loader)? {
                return Err(Error::ClassNotFound {
                    name: class_name.to_string(),
                });
            }
            registry.resolve_loaded_class_key(class_name)?
        }
        Err(err) => return Err(err),
    };
    let entry_idx = {
        let ctx = registry.get(&class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| Error::MethodNotFound {
                name: format!("{class_name}.{method_name}"),
                descriptor: descriptor.to_string(),
            })?
    };
    let class_loader = registry.class_loader(&class_name).cloned();
    let init_loader = class_loader.as_deref().map_or(loader, |v| v);
    ensure_initialized(registry, init_loader, heap, stdout, &class_name, "")?;
    ExecutionState::new(registry, &class_name, method_name, entry_idx, args)
}
fn prepare_lambda_executor_invocation(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    task_ref: u64,
    method: &str,
    descriptor: &str,
) -> Result<Option<ExecutorInvocation>> {
    let task_class = heap.get(task_ref)?.class_name.clone();
    let Some(lambda_info) = registry.get_lambda(&task_class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }
    let lambda_object = heap.get(task_ref)?;
    let mut impl_args = Vec::with_capacity(lambda_info.captured_count);
    for capture_index in 0..lambda_info.captured_count {
        impl_args
            .push(
                lambda_object
                    .fields
                    .get(capture_index)
                    .copied()
                    .ok_or(Error::Unimplemented {
                        mnemonic: "lambda capture missing",
                    })?,
            );
    }
    let _ = registry
        .ensure_loaded_from(&lambda_info.impl_class, Some(task_class.as_str()), loader);
    let impl_class_key = registry
        .class_key_from_source(&lambda_info.impl_class, Some(task_class.as_str()));
    let dispatch_class = match lambda_info.impl_kind {
        6 | 7 => impl_class_key,
        5 | 9 => {
            match impl_args.first().copied() {
                Some(Slot::Reference(Some(receiver_ref))) => {
                    let receiver_class = heap.get(receiver_ref)?.class_name.clone();
                    if let Some((dispatch_class, _)) = resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &receiver_class,
                        &lambda_info.impl_method,
                        &lambda_info.impl_desc,
                    ) {
                        dispatch_class
                    } else {
                        impl_class_key
                    }
                }
                Some(Slot::Reference(None)) | None => {
                    return Err(Error::NullPointerException);
                }
                Some(_) => {
                    return Err(Error::TypeMismatch {
                        expected: "reference",
                        got: "other",
                    });
                }
            }
        }
        _ => {
            return Err(Error::Unimplemented {
                mnemonic: "executor lambda impl kind",
            });
        }
    };
    ensure_initialized(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        task_class.as_str(),
    )?;
    let (_, method_idx) = resolve_method_in_hierarchy(
            registry,
            loader,
            &dispatch_class,
            &lambda_info.impl_method,
            &lambda_info.impl_desc,
        )
        .ok_or_else(|| Error::MethodNotFound {
            name: format!("{}.{}", dispatch_class, lambda_info.impl_method),
            descriptor: lambda_info.impl_desc.clone(),
        })?;
    let impl_args = adapt_args_for_impl_desc(&impl_args, &lambda_info.impl_desc, heap);
    let state = ExecutionState::new(
        registry,
        &dispatch_class,
        &lambda_info.impl_method,
        method_idx,
        &impl_args,
    )?;
    Ok(
        Some(ExecutorInvocation {
            state,
            impl_desc: Some(lambda_info.impl_desc),
            sam_desc: Some(lambda_info.sam_desc),
        }),
    )
}
fn prepare_executor_invocation(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    task: duke_gc::ExecutorTask,
) -> Result<ExecutorInvocation> {
    let (method, descriptor) = executor_task_signature(task.kind);
    if let Some(invocation) = prepare_lambda_executor_invocation(
        registry,
        loader,
        heap,
        output,
        task.task_ref,
        method,
        descriptor,
    )? {
        return Ok(invocation);
    }
    let task_class = heap.get(task.task_ref)?.class_name.clone();
    let (dispatch_class, method_idx) = resolve_method_in_hierarchy(
            registry,
            loader,
            &task_class,
            method,
            descriptor,
        )
        .ok_or_else(|| Error::AbstractMethodError {
            class_name: task_class.clone(),
            method_name: method.to_string(),
        })?;
    ensure_initialized(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        task_class.as_str(),
    )?;
    let state = ExecutionState::new(
        registry,
        &dispatch_class,
        method,
        method_idx,
        &[Slot::Reference(Some(task.task_ref))],
    )?;
    Ok(ExecutorInvocation {
        state,
        impl_desc: None,
        sam_desc: None,
    })
}
fn render_properties_store(
    heap: &duke_gc::Heap,
    props_ref: u64,
    comment: Option<&str>,
) -> Result<String> {
    let mut output = String::new();
    if let Some(comment_text) = comment {
        append_store_comment(&mut output, comment_text);
    }
    output.push_str("# ");
    output.push_str(PROPERTIES_STORE_TIMESTAMP);
    output.push('\n');
    for (key, value) in properties_local_string_entries(heap, props_ref)? {
        output.push_str(&escape_property_key(&key));
        output.push('=');
        output.push_str(&escape_property_value(&value));
        output.push('\n');
    }
    Ok(output)
}
pub(crate) fn native_paths_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first_ref = extract_ref_arg(args, 0)?;
    let mut path = std::path::PathBuf::from(string_value_from_ref(heap, first_ref)?);
    let more_slot = extract_slot_arg(args, 1);
    match more_slot {
        Slot::Reference(Some(array_ref)) => {
            let segments = heap.get(array_ref)?.fields.clone();
            for segment in segments {
                let Slot::Reference(Some(segment_ref)) = segment else {
                    return Err(Error::NullPointerException);
                };
                path.push(string_value_from_ref(heap, segment_ref)?);
            }
        }
        Slot::Reference(None) => {}
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().into_owned(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}
fn fill_throwable_stack_trace_from_control(
    heap: &mut duke_gc::Heap,
    throwable_ref: u64,
    control: &NativeControl,
) -> Result<()> {
    store_throwable_stack_trace_from_frames(heap, throwable_ref, control.stack_trace())
}
pub(crate) fn native_protection_domain_get_code_source(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(Error::NullPointerException),
    }
}
fn current_host_thread_is_interrupted() -> bool {
    interrupted_host_threads()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains(&current_host_thread_id())
}
fn current_host_thread_id() -> std::thread::ThreadId {
    std::thread::current().id()
}
/// Native: `SecureRandom.nextBytes([B)V` — fills target array using host CSPRNG.
pub(crate) fn native_secure_random_next_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let array_ref = extract_ref_arg(args, 1)?;
    let mut bytes = vec![0_u8; heap.get(array_ref) ?.fields.len()];
    getrandom::fill(&mut bytes)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/InternalError".to_string(),
        })?;
    let arr = heap.get_mut(array_ref)?;
    for (idx, byte) in bytes.iter().copied().enumerate() {
        arr.fields[idx] = Slot::Int(i32::from(byte));
    }
    Ok(None)
}
/// Native: `SecureRandom.generateSeed(I)[B` — returns a fresh random byte array.
pub(crate) fn native_secure_random_generate_seed(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let len = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let mut bytes = vec![0_u8; len];
    getrandom::fill(&mut bytes)
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/InternalError".to_string(),
        })?;
    let out_ref = alloc_byte_array(heap, &bytes);
    Ok(Some(Slot::Reference(Some(out_ref))))
}
/// Native: `Function.andThen(Function)Function` — `f.andThen(g)` = `g(f(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let first = extract_slot_arg(args, 0);
    let second = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndThenFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Function.compose(Function)Function` — `f.compose(g)` = `f(g(x))`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_function_compose(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let outer = extract_slot_arg(args, 0);
    let inner = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/ComposeFunction".to_string(), 2);
    heap.get_mut(r)?.fields[0] = outer;
    heap.get_mut(r)?.fields[1] = inner;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `ComparingIntComparator.compare(O,O)I` — calls `fn.applyAsInt(o)` for each element.
pub(crate) fn native_comparing_int_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), a],
        )?
        .unwrap_or(Slot::Int(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsInt",
            "(Ljava/lang/Object;)I",
            vec![Slot::Reference(Some(fn_ref)), b],
        )?
        .unwrap_or(Slot::Int(0));
    let result = match (ka, kb) {
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    let _ = control;
    Ok(Some(Slot::Int(result)))
}
/// Native: `Comparator.comparing compare(Object,Object)I` — compare via key extractor.
pub(crate) fn native_comparing_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Reference(None));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Reference(None));
    let cmp = compare_treemap_keys(ka, kb, heap) as i32;
    Ok(Some(Slot::Int(cmp)))
}
/// Native: `ComparingLongComparator.compare(O,O)I` — calls `fn.applyAsLong(o)` for each element.
pub(crate) fn native_comparing_long_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Long(0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsLong",
            "(Ljava/lang/Object;)J",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Long(0));
    let result = match (ka, kb) {
        (Slot::Long(la), Slot::Long(lb)) => la.cmp(&lb) as i32,
        (Slot::Int(ia), Slot::Int(ib)) => ia.cmp(&ib) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}
/// Native: `ComparingDoubleComparator.compare(O,O)I` — calls `fn.applyAsDouble(o)` for each.
pub(crate) fn native_comparing_double_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fn_slot = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(fn_ref)) = fn_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let fn_class = heap.get(fn_ref)?.class_name.clone();
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let ka = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, a],
        )?
        .unwrap_or(Slot::Double(0.0));
    let kb = ops
        .invoke(
            heap,
            out,
            &fn_class,
            "applyAsDouble",
            "(Ljava/lang/Object;)D",
            vec![fn_slot, b],
        )?
        .unwrap_or(Slot::Double(0.0));
    let result = match (ka, kb) {
        (Slot::Double(da), Slot::Double(db)) => da.total_cmp(&db) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(result)))
}
fn interrupted_host_threads() -> &'static RwLock<HashSet<std::thread::ThreadId>> {
    static INTERRUPTED: OnceLock<RwLock<HashSet<std::thread::ThreadId>>> = OnceLock::new();
    INTERRUPTED.get_or_init(|| RwLock::new(HashSet::new()))
}
fn interrupted_exception_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/InterruptedException".to_string(),
    }
}
/// Native: `ReversedComparator.compare(a, b)I` — inverts delegate comparison.
pub(crate) fn native_reversed_comparator_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let delegate = extract_first_field_arg(heap, this_ref)?;
    let Slot::Reference(Some(del_ref)) = delegate else {
        return Ok(Some(Slot::Int(0)));
    };
    let del_class = heap.get(del_ref)?.class_name.clone();
    let result = ops
        .invoke(
            heap,
            out,
            &del_class,
            "compare",
            "(Ljava/lang/Object;Ljava/lang/Object;)I",
            vec![delegate, a, b],
        )?
        .unwrap_or(Slot::Int(0));
    let cmp = match result {
        Slot::Int(n) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(-cmp)))
}
/// Index of a named static field within `ctx.static_fields`.
fn static_field_idx(ctx: &ClassContext, name: &str) -> Result<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(Error::InvalidFieldref { index: 0 })
}
/// Native: `BiFunction.andThen(Function)BiFunction` — returns `BiFunctionAndThen` proxy.
pub(crate) fn native_bifunction_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let bifunction = extract_slot_arg(args, 0);
    let after = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/BiFunctionAndThen".to_string(), 2);
    heap.get_mut(r)?.fields[0] = bifunction;
    heap.get_mut(r)?.fields[1] = after;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `BiFunctionAndThen.apply(Object,Object)Object` — calls wrapped bifunction then after.
pub(crate) fn native_bifunction_and_then_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let bifunction = extract_first_field_arg(heap, this_ref)?;
    let after = extract_field_arg(heap, this_ref, 1)?;
    let Slot::Reference(Some(bf_ref)) = bifunction else {
        return Ok(Some(Slot::Reference(None)));
    };
    let bf_class = heap.get(bf_ref)?.class_name.clone();
    let mid = ops
        .invoke(
            heap,
            out,
            &bf_class,
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![bifunction, a, b],
        )?
        .unwrap_or(Slot::Reference(None));
    Ok(Some(invoke_function_apply(after, mid, heap, out, ops)?))
}
fn cp_utf8_string(cp: &[Option<CpEntry>], cp_idx: usize) -> Result<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => {
            Err(Error::InvalidCpIndex {
                index: cp_idx,
            })
        }
    }
}
fn cp_annotation_const(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> Option<ReflectedAnnotationConst> {
    match cp.get(cp_idx).and_then(|e| e.as_ref())? {
        CpEntry::Integer(value) => Some(ReflectedAnnotationConst::Int(*value)),
        CpEntry::Long(value) => Some(ReflectedAnnotationConst::Long(*value)),
        CpEntry::Float(value) => Some(ReflectedAnnotationConst::Float(*value)),
        CpEntry::Double(value) => Some(ReflectedAnnotationConst::Double(*value)),
        CpEntry::Utf8(value) => Some(ReflectedAnnotationConst::String(value.clone())),
        _ => None,
    }
}
/// Native: `ComposeFunction.apply(O)O` — applies inner then outer.
pub(crate) fn native_compose_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let outer = extract_first_field_arg(heap, this_ref)?;
    let inner = extract_field_arg(heap, this_ref, 1)?;
    let mid = invoke_function_apply(inner, input, heap, out, ops)?;
    invoke_function_apply(outer, mid, heap, out, ops).map(Some)
}
fn reflected_constructors(
    reflected: ReflectedClassInfo,
    public_only: bool,
) -> Vec<ReflectedMethodInfo> {
    reflected
        .methods
        .into_iter()
        .filter(|method| method.name == "<init>" && (!public_only || method.is_public))
        .collect()
}
fn reflected_class_info_from_loader(
    loader: &dyn ClassLoader,
    internal_name: &str,
) -> Option<ReflectedClassInfo> {
    use duke_classfile::{FieldAccessFlags, MethodAccessFlags};
    let bytes = loader.find_class(internal_name).ok()?;
    let class_file = duke_classfile::parse(&bytes).ok()?;
    let methods = class_file
        .methods
        .iter()
        .filter_map(|method| {
            let name = cp_utf8_string(
                    &class_file.constant_pool,
                    method.name_index.0 as usize,
                )
                .ok()?;
            let descriptor = cp_utf8_string(
                    &class_file.constant_pool,
                    method.descriptor_index.0 as usize,
                )
                .ok()?;
            Some(ReflectedMethodInfo {
                name,
                descriptor,
                is_public: method.access_flags.contains(MethodAccessFlags::PUBLIC),
                is_static: method.access_flags.contains(MethodAccessFlags::STATIC),
                annotations: runtime_visible_annotations_from_attrs(
                    &class_file.constant_pool,
                    &method.attributes,
                ),
                annotation_default: annotation_default_from_attrs(
                    &class_file.constant_pool,
                    &method.attributes,
                ),
            })
        })
        .collect();
    let fields = class_file
        .fields
        .iter()
        .filter_map(|field| {
            let name = cp_utf8_string(
                    &class_file.constant_pool,
                    field.name_index.0 as usize,
                )
                .ok()?;
            let descriptor = cp_utf8_string(
                    &class_file.constant_pool,
                    field.descriptor_index.0 as usize,
                )
                .ok()?;
            Some(ReflectedFieldInfo {
                name,
                descriptor,
                is_public: field.access_flags.contains(FieldAccessFlags::PUBLIC),
                is_static: field.access_flags.contains(FieldAccessFlags::STATIC),
                annotations: runtime_visible_annotations_from_attrs(
                    &class_file.constant_pool,
                    &field.attributes,
                ),
            })
        })
        .collect();
    let super_class = if class_file.super_class.0 != 0 {
        resolve_class_name(&class_file.constant_pool, class_file.super_class.0 as usize)
            .ok()
    } else {
        None
    };
    let interfaces = class_file
        .interfaces
        .iter()
        .filter_map(|idx| {
            resolve_class_name(&class_file.constant_pool, idx.0 as usize).ok()
        })
        .collect();
    Some(ReflectedClassInfo {
        internal_name: internal_name.to_string(),
        binary_name: internal_name_to_binary_name(internal_name),
        super_class,
        interfaces,
        methods,
        fields,
        annotations: runtime_visible_annotations_from_attrs(
            &class_file.constant_pool,
            &class_file.attributes,
        ),
    })
}
fn reflected_field_handle(
    heap: &duke_gc::Heap,
    field_ref: u64,
) -> Result<ReflectedFieldHandle> {
    let field_obj = heap.get(field_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: field_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: field_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = field_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: field_ref,
        });
    };
    let is_public = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD), Some(Slot::Int(value)) if *
        value != 0
    );
    let is_static = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD), Some(Slot::Int(value)) if *
        value != 0
    );
    let is_accessible = matches!(
        field_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD), Some(Slot::Int(value))
        if * value != 0
    );
    Ok(ReflectedFieldHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        field_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}
fn reflected_method_handle(
    heap: &duke_gc::Heap,
    method_ref: u64,
) -> Result<ReflectedMethodHandle> {
    let method_obj = heap.get(method_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied() else {
        return Err(Error::InvalidRef {
            address: method_ref,
        });
    };
    let is_public = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD), Some(Slot::Int(value)) if
        * value != 0
    );
    let is_static = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD), Some(Slot::Int(value)) if
        * value != 0
    );
    let is_accessible = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_ACCESSIBLE_FIELD), Some(Slot::Int(value))
        if * value != 0
    );
    Ok(ReflectedMethodHandle {
        declaring_class_key: class_key_from_ref(heap, declaring_class_ref)?,
        method_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(Error::NullPointerException)?,
        is_public,
        is_static,
        is_accessible,
    })
}
/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}
/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}
/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}
/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap.get(class_ref)?.string_value.clone().unwrap_or_default();
    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }
    Err(Error::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}
fn binary_name_to_internal_name(name: &str) -> String {
    name.replace('.', "/")
}
fn validate_pattern_flags(flags: i32) -> Result<()> {
    if flags & !PATTERN_SUPPORTED_FLAGS != 0 {
        return Err(regex_illegal_argument(format!("Unknown regex flags: {flags}")));
    }
    if flags & PATTERN_CANON_EQ != 0 {
        return Err(
            regex_pattern_syntax_error(
                "CANON_EQ is not supported by Duke's regex engine",
            ),
        );
    }
    Ok(())
}
fn chm_non_null_arg(args: &[Slot], idx: usize) -> Result<Slot> {
    require_chm_non_null(extract_slot_arg(args, idx))
}
fn chm_entry_snapshot(heap: &duke_gc::Heap, map_ref: u64) -> Result<Vec<(Slot, Slot)>> {
    let fields = heap.get(map_ref)?.fields.clone();
    let mut entries = Vec::with_capacity(fields.len().saturating_sub(1) / 2);
    let mut i = 1usize;
    while i + 1 < fields.len() {
        entries.push((fields[i], fields[i + 1]));
        i += 2;
    }
    Ok(entries)
}
/// Helper: compile a regex from a pattern string.
/// Returns `Err` with `JavaException` on bad pattern.
fn compile_java_regex(pattern: &str) -> Result<regex::Regex> {
    compile_java_regex_with_flags(pattern, 0)
}
fn compile_java_regex_with_flags(pattern: &str, flags: i32) -> Result<regex::Regex> {
    validate_pattern_flags(flags)?;
    let mut source = if flags & PATTERN_LITERAL != 0 {
        regex::escape(pattern)
    } else {
        translate_java_named_groups(pattern)
    };
    let ascii_case_insensitive = flags & PATTERN_CASE_INSENSITIVE != 0
        && flags & PATTERN_UNICODE_CASE == 0;
    if ascii_case_insensitive {
        source = expand_ascii_case_insensitive(&source);
    }
    let mut builder = regex::RegexBuilder::new(&source);
    builder
        .case_insensitive(
            flags & PATTERN_CASE_INSENSITIVE != 0 && !ascii_case_insensitive,
        )
        .multi_line(flags & PATTERN_MULTILINE != 0)
        .dot_matches_new_line(flags & PATTERN_DOTALL != 0)
        .ignore_whitespace(flags & PATTERN_COMMENTS != 0)
        .unicode(true);
    builder.build().map_err(|e| regex_pattern_syntax_error(e.to_string()))
}
/// Native: `Predicate.and(Predicate)Predicate` — logical AND of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_and(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/AndPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Predicate.or(Predicate)Predicate` — logical OR of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let left = extract_slot_arg(args, 0);
    let right = extract_slot_arg(args, 1);
    let r = heap.allocate("duke/util/OrPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Predicate.negate()Predicate` — logical NOT of a predicate.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_negate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let original = extract_slot_arg(args, 0);
    let r = heap.allocate("duke/util/NegatedPredicate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = original;
    Ok(Some(Slot::Reference(Some(r))))
}
fn full_byte_array(heap: &duke_gc::Heap, array_ref: u64) -> Result<Vec<u8>> {
    let length = i32::try_from(heap.get(array_ref)?.fields.len())
        .map_err(|_| index_out_of_bounds_error())?;
    byte_array_window(heap, array_ref, 0, length)
}
/// Native: `NaturalOrderComparator.compare(O,O)I` — delegates to `o1.compareTo(o2)`.
pub(crate) fn native_natural_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let o1 = extract_slot_arg(args, 1);
    let o2 = extract_slot_arg(args, 2);
    let o1_class = match &o1 {
        Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let result = ops
        .invoke(
            heap,
            out,
            &o1_class,
            "compareTo",
            "(Ljava/lang/Object;)I",
            vec![o1, o2],
        )?;
    let _ = control;
    Ok(Some(result.unwrap_or(Slot::Int(0))))
}
/// Native: `Security.getProvider(String)Provider`.
pub(crate) fn native_security_get_provider(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let name = string_value_from_ref(heap, name_ref)?;
    if !is_duke_provider_name(&name) {
        return Ok(Some(Slot::Reference(None)));
    }
    let provider_ref = allocate_duke_provider(heap);
    Ok(Some(Slot::Reference(Some(provider_ref))))
}
/// Native: `Security.getProviders()Provider[]`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_security_get_providers(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let provider_ref = allocate_duke_provider(heap);
    let provider_array_ref = heap.allocate("[Ljava/security/Provider;".to_string(), 1);
    if let Ok(providers) = heap.get_mut(provider_array_ref) {
        providers.fields[0] = Slot::Reference(Some(provider_ref));
    }
    Ok(Some(Slot::Reference(Some(provider_array_ref))))
}
/// Native: `Security.getAlgorithms(String)Set`.
pub(crate) fn native_security_get_algorithms(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let service_type_ref = extract_ref_arg(args, 0)?;
    let service_type = string_value_from_ref(heap, service_type_ref)?;
    let algorithms = if is_message_digest_service_type(&service_type) {
        &MESSAGE_DIGEST_ALGORITHMS[..]
    } else {
        &[][..]
    };
    let set_ref = allocate_algorithm_set(heap, out, control, algorithms)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}
/// Native: `List.of(Object...)List` — all args (fixed-arity or varargs) become a new `ArrayList`.
///
/// Handles descriptors with 0–6+ fixed args and the varargs `([O)List` form.
pub(crate) fn native_list_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let list_ref = make_list_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(list_ref))))
}
fn list_directory_children_sorted(
    path: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>> {
    let iter = std::fs::read_dir(path)
        .map_err(|_| Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;
    let mut children = Vec::new();
    for entry in iter {
        let entry = entry
            .map_err(|_| Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            })?;
        children.push(entry.path());
    }
    children
        .sort_by(|left, right| {
            left.file_name()
                .unwrap_or_default()
                .cmp(right.file_name().unwrap_or_default())
        });
    Ok(children)
}
fn logical_properties_lines(text: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut pending = String::new();
    let mut continuing = false;
    for line in split_properties_lines(text) {
        let mut piece = if continuing {
            line.trim_start_matches(is_properties_whitespace).to_string()
        } else {
            line
        };
        if continuing && piece.is_empty() {
            continue;
        }
        if has_odd_trailing_backslashes(&piece) {
            piece.pop();
            pending.push_str(&piece);
            continuing = true;
        } else {
            pending.push_str(&piece);
            logical.push(std::mem::take(&mut pending));
            continuing = false;
        }
    }
    if continuing || !pending.is_empty() {
        logical.push(pending);
    }
    logical
}
fn append_throwable_trace(
    heap: &duke_gc::Heap,
    throwable_ref: u64,
    caption: &str,
    frame_indent: &str,
    out: &mut String,
    visited: &mut std::collections::HashSet<u64>,
) -> Result<()> {
    if !visited.insert(throwable_ref) {
        out.push_str(caption);
        out.push_str("[CIRCULAR REFERENCE: ");
        out.push_str(&throwable_header(heap, throwable_ref)?);
        out.push_str("]\n");
        return Ok(());
    }
    out.push_str(caption);
    out.push_str(&throwable_header(heap, throwable_ref)?);
    out.push('\n');
    let stack_slot = throwable_field_slot(
        heap,
        throwable_ref,
        THROWABLE_STACK_TRACE_FIELD,
    )?;
    if let Some(stack_ref) = stack_slot.as_reference() {
        for frame_slot in &heap.get(stack_ref)?.fields {
            if let Slot::Reference(Some(element_ref)) = frame_slot {
                out.push_str(frame_indent);
                out.push_str("\tat ");
                out.push_str(&stack_trace_element_text(heap, *element_ref)?);
                out.push('\n');
            }
        }
    }
    let suppressed_slot = throwable_field_slot(
        heap,
        throwable_ref,
        THROWABLE_SUPPRESSED_FIELD,
    )?;
    if let Some(suppressed_ref) = suppressed_slot.as_reference() {
        for suppressed in &heap.get(suppressed_ref)?.fields {
            if let Slot::Reference(Some(suppressed_ref)) = suppressed {
                let mut suppressed_caption = String::from(frame_indent);
                suppressed_caption.push_str("\tSuppressed: ");
                let mut suppressed_indent = String::from(frame_indent);
                suppressed_indent.push('\t');
                append_throwable_trace(
                    heap,
                    *suppressed_ref,
                    &suppressed_caption,
                    &suppressed_indent,
                    out,
                    visited,
                )?;
            }
        }
    }
    let cause_slot = throwable_field_slot(heap, throwable_ref, THROWABLE_CAUSE_FIELD)?;
    if let Some(cause_ref) = cause_slot.as_reference() && cause_ref != throwable_ref {
        append_throwable_trace(
            heap,
            cause_ref,
            "Caused by: ",
            frame_indent,
            out,
            visited,
        )?;
    }
    Ok(())
}
fn append_message_digest_buffer(
    heap: &mut duke_gc::Heap,
    digest_ref: u64,
    bytes: &[u8],
) -> Result<()> {
    let mut buffer = message_digest_buffer(heap, digest_ref)?;
    buffer.extend_from_slice(bytes);
    write_message_digest_buffer(heap, digest_ref, &buffer)
}
fn append_to_string_builder(
    heap: &mut duke_gc::Heap,
    builder_ref: u64,
    text: &str,
) -> Result<()> {
    let builder = heap.get_mut(builder_ref)?;
    builder.string_value.get_or_insert_with(String::new).push_str(text);
    Ok(())
}
fn append_store_comment(out: &mut String, comment: &str) {
    for line in split_properties_lines(comment) {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            out.push_str("# ");
            out.push_str(&escape_property_comment(&line));
            out.push('\n');
        }
    }
}
/// Pop args from the operand stack and store them in the locals array.
///
/// Wide types (J = long, D = double) occupy two local variable slots in the JVM.
/// For each such type, the value is stored at `local_idx` and `local_idx + 1` is
/// left as the default padding (`Slot::Int(0)`).
///
/// For methods without any wide-type params, this behaves identically to the
/// previous simple sequential assignment.
fn pop_typed_args_into_locals(
    param_types: &[char],
    frame: &mut Frame,
    locals: &mut [Slot],
    start_idx: usize,
) -> Result<()> {
    let count = param_types.len();
    let mut args = vec![Slot::Int(0); count];
    for i in (0..count).rev() {
        args[i] = frame.pop()?;
    }
    let mut local_idx = start_idx;
    for (slot, &tc) in args.iter().zip(param_types.iter()) {
        if local_idx < locals.len() {
            locals[local_idx] = *slot;
        }
        local_idx += 1;
        if tc == 'J' || tc == 'D' {
            local_idx += 1;
        }
    }
    Ok(())
}
fn pop_pending_java_exception_message(class_name: &str) -> Option<String> {
    let mut messages = pending_java_exception_messages()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let queue = messages.get_mut(class_name)?;
    let message = queue.pop_front();
    if queue.is_empty() {
        messages.remove(class_name);
    }
    message
}
fn pop_pending_java_exception_cause(class_name: &str) -> Option<Slot> {
    let mut causes = pending_java_exception_causes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let queue = causes.get_mut(class_name)?;
    let cause = queue.pop_front();
    if queue.is_empty() {
        causes.remove(class_name);
    }
    cause
}
fn normalize_resource_name(name: &str) -> Option<String> {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\')
        || name.contains(':')
    {
        return None;
    }
    let mut normalized = String::with_capacity(name.len());
    for (idx, component) in name.split(['/', '\\']).enumerate() {
        if component.is_empty() || component == "." || component == ".." {
            return None;
        }
        if idx > 0 {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    Some(normalized)
}
fn normalize_seconds_nanos(total_nanos: i128) -> (i64, i32) {
    let seconds = clamp_i128_to_i64(total_nanos.div_euclid(NANOS_PER_SECOND_I128));
    let nanos = i32::try_from(total_nanos.rem_euclid(NANOS_PER_SECOND_I128))
        .unwrap_or(0);
    (seconds, nanos)
}
fn clone_reference_array(
    heap: &mut duke_gc::Heap,
    slot: Slot,
    default_class_name: &str,
) -> Result<Slot> {
    let Some(array_ref) = slot.as_reference() else {
        let empty_ref = allocate_empty_reference_array(heap, default_class_name)?;
        return Ok(Slot::Reference(Some(empty_ref)));
    };
    let (class_name, elements) = {
        let obj = heap.get(array_ref)?;
        (obj.class_name.clone(), obj.fields.clone())
    };
    let cloned_ref = allocate_slot_array(heap, &class_name, &elements)?;
    Ok(Slot::Reference(Some(cloned_ref)))
}
fn uncaught_java_exception_refs() -> &'static UncaughtExceptionRefs {
    static REFS: OnceLock<UncaughtExceptionRefs> = OnceLock::new();
    REFS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}
/// Helper: days in a given month of a given year (handles leap years).
const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 29 } else { 28 }
        }
        _ => 30,
    }
}
#[cfg(feature = "telemetry")]
fn refresh_current_method_name(
    current_method: &mut String,
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
) {
    *current_method = registry
        .get(current_class)
        .map(|c| {
            c.methods.get(method_idx).map(|m| m.name.clone()).unwrap_or_default()
        })
        .unwrap_or_default();
}
fn line_number_for_bci(line_number_table: &[(u16, u16)], bci: usize) -> i32 {
    line_number_table
        .iter()
        .filter(|(start_pc, _)| usize::from(*start_pc) <= bci)
        .max_by_key(|(start_pc, _)| *start_pc)
        .map_or(-1, |(_, line)| i32::from(*line))
}
fn set_object_field(
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    index: usize,
    value: Slot,
) -> Result<()> {
    let obj = heap.get_mut(obj_ref)?;
    if obj.fields.len() <= index {
        obj.fields.resize(index + 1, Slot::Reference(None));
    }
    obj.fields[index] = value;
    Ok(())
}
/// Native: `Set.of(Object...)Set` — all args (fixed-arity or varargs) become a new `HashSet`.
///
/// Handles both fixed-arity descriptors (multiple direct element args)
/// and the single-array varargs form `([O)Set`.
pub(crate) fn native_set_of_factory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let elems: Vec<Slot> = if args.len() == 1 {
        if let Some(Slot::Reference(Some(arr_ref))) = args.first() {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                let fields = obj.fields.clone();
                let _ = obj;
                fields
            } else {
                args.to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        args.to_vec()
    };
    let set_ref = make_set_from_slots(&elems, heap, out, control)?;
    Ok(Some(Slot::Reference(Some(set_ref))))
}
fn set_matcher_no_match(heap: &mut duke_gc::Heap, m_ref: u64) -> Result<()> {
    let matcher = heap.get_mut(m_ref)?;
    matcher.fields[MATCHER_MATCH_START_FIELD] = Slot::Int(-1);
    matcher.fields[MATCHER_MATCH_END_FIELD] = Slot::Int(0);
    matcher.string_value = None;
    Ok(())
}
pub(crate) fn native_set_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;
    match extract_slot_arg(args, 0) {
        Slot::Reference(Some(array_ref)) => {
            let elements = heap.get(array_ref)?.fields.clone();
            for element in elements {
                native_hashset_add(
                    &[Slot::Reference(Some(set_ref)), element],
                    heap,
                    out,
                    control,
                )?;
            }
        }
        Slot::Reference(None) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    Ok(Some(Slot::Reference(Some(set_ref))))
}
fn has_registered_native_override(
    registry: &ClassRegistry,
    class_name: &str,
    method_name: &str,
    method_desc: &str,
) -> bool {
    lookup_registered_native_kind(registry, class_name, method_name, method_desc)
        .is_some()
}
fn has_odd_trailing_backslashes(line: &str) -> bool {
    let mut count = 0usize;
    for ch in line.chars().rev() {
        if ch != '\\' {
            break;
        }
        count += 1;
    }
    count % 2 == 1
}
fn materialize_annotation_const(
    heap: &mut duke_gc::Heap,
    descriptor: &str,
    value: &ReflectedAnnotationConst,
) -> Slot {
    match value {
        ReflectedAnnotationConst::String(value) => {
            Slot::Reference(Some(heap.allocate_string(value.clone())))
        }
        ReflectedAnnotationConst::Long(value) => Slot::Long(*value),
        ReflectedAnnotationConst::Float(value) => Slot::Float(*value),
        ReflectedAnnotationConst::Double(value) => Slot::Double(*value),
        ReflectedAnnotationConst::Boolean(value) => Slot::Int(i32::from(*value)),
        ReflectedAnnotationConst::Byte(value)
        | ReflectedAnnotationConst::Char(value)
        | ReflectedAnnotationConst::Int(value)
        | ReflectedAnnotationConst::Short(value) => {
            if descriptor == "Z" {
                Slot::Int(i32::from(*value != 0))
            } else {
                Slot::Int(*value)
            }
        }
    }
}
fn materialize_annotation_value(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    descriptor: &str,
    value: &ReflectedAnnotationValue,
) -> Result<Slot> {
    match value {
        ReflectedAnnotationValue::Const(value) => {
            Ok(materialize_annotation_const(heap, descriptor, value))
        }
        ReflectedAnnotationValue::Class(class_key) => {
            let class_ref = allocate_class_object(heap, class_key)?;
            Ok(Slot::Reference(Some(class_ref)))
        }
        ReflectedAnnotationValue::Enum { type_name, const_name } => {
            ops.ensure_class_initialized(heap, output, type_name)?;
            ops.read_static_field(type_name, const_name)
        }
        ReflectedAnnotationValue::Annotation(annotation) => {
            let annotation_ref = allocate_annotation_proxy(
                heap,
                output,
                ops,
                annotation,
            )?;
            Ok(Slot::Reference(Some(annotation_ref)))
        }
        ReflectedAnnotationValue::Array(values) => {
            let element_descriptor = array_element_descriptor(descriptor);
            let slots = values
                .iter()
                .map(|value| {
                    materialize_annotation_value(
                        heap,
                        output,
                        ops,
                        element_descriptor,
                        value,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let array_ref = allocate_reference_array_from_slots(
                heap,
                descriptor,
                &slots,
            )?;
            Ok(Slot::Reference(Some(array_ref)))
        }
    }
}
/// Allocate a heap exception object for a native-thrown Java exception.
///
/// Native handlers currently surface Java exceptions as class names. Catch
/// blocks need an object reference on the operand stack, so we materialize a
/// minimal heap object of that class before routing through exception-table
/// dispatch.
fn materialize_java_exception_object(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> Result<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap
        .allocate(
            class_name.to_string(),
            total_instance_field_count(registry, class_name),
        );
    init_object_fields(registry, heap, exc_ref, class_name);
    if let Some(message) = pop_pending_java_exception_message(class_name) {
        heap.get_mut(exc_ref)?.string_value = Some(message);
    }
    if let Some(cause) = pop_pending_java_exception_cause(class_name)
        && let Ok(obj) = heap.get_mut(exc_ref) && !obj.fields.is_empty()
    {
        obj.fields[THROWABLE_CAUSE_FIELD] = cause;
        heap.remember_reference_write(exc_ref, cause);
    }
    Ok(exc_ref)
}
pub(crate) fn record_uncaught_java_exception_ref(class_name: &str, exception_ref: u64) {
    let mut refs = uncaught_java_exception_refs()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    refs.entry(std::thread::current().id())
        .or_default()
        .push_back((class_name.to_string(), exception_ref));
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
fn field_slot_idx(
    registry: &ClassRegistry,
    target_class: &str,
    name: &str,
) -> Result<usize> {
    let mut current = target_class;
    while let Ok(ctx) = registry.get(current) {
        if let Some(local_idx) = ctx
            .fields
            .iter()
            .filter(|f| !f.is_static)
            .position(|f| f.name == name)
        {
            let super_fields = ctx
                .super_class
                .as_deref()
                .map_or(0, |sc| total_instance_field_count(registry, sc));
            return Ok(super_fields + local_idx);
        }
        if let Some(ref sc) = ctx.super_class {
            current = sc;
        } else {
            break;
        }
    }
    Err(Error::InvalidFieldref { index: 0 })
}
/// Semantic equality for `HashMap` keys: reference identity by default, with value
/// semantics for strings, class mirrors, and boxed primitive wrappers.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name != ob.class_name {
                return false;
            }
            match oa.class_name.as_str() {
                "java/lang/String" | "java/lang/Class" => {
                    oa.string_value == ob.string_value
                }
                "java/util/UUID" => {
                    uuid_bits_from_object(oa) == uuid_bits_from_object(ob)
                }
                class_name if uses_first_field_value_equality(class_name) => {
                    oa.fields.first() == ob.fields.first()
                }
                _ => false,
            }
        }
        _ => false,
    }
}
fn register_java_host_thread(host_key: i32, host_thread_id: std::thread::ThreadId) {
    java_thread_hosts()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(host_key, host_thread_id);
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
        Some('L' | '[') => Slot::Reference(None),
        _ => Slot::Int(0),
    }
}
fn native_needs_stack_snapshot(
    registry: &ClassRegistry,
    native_class: &str,
    native_method: &str,
    native_desc: &str,
) -> bool {
    (native_method == "fillInStackTrace" && native_desc == "()Ljava/lang/Throwable;")
        || (native_method == "<init>"
            && matches!(
                native_desc, "()V" | "(Ljava/lang/String;)V" |
                "(Ljava/lang/String;Ljava/lang/Throwable;)V"
            ) && is_throwable_class(registry, native_class))
}
/// Native: `AndPredicate.test(O)Z` — both predicates must return true.
pub(crate) fn native_and_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if !la {
        return Ok(Some(Slot::Int(0)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}
/// Native: `AndThenFunction.apply(O)O` — applies first then second.
pub(crate) fn native_and_then_function_apply(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let input = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    let mid = invoke_function_apply(first, input, heap, out, ops)?;
    invoke_function_apply(second, mid, heap, out, ops).map(Some)
}
/// Native: `AndThenConsumer.accept(O)V` — runs first then second consumer.
pub(crate) fn native_and_then_consumer_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let arg = extract_slot_arg(args, 1);
    let first = extract_first_field_arg(heap, this_ref)?;
    let second = extract_field_arg(heap, this_ref, 1)?;
    invoke_consumer_accept(first, arg, heap, out, ops)?;
    invoke_consumer_accept(second, arg, heap, out, ops)?;
    Ok(None)
}
#[allow(clippy::too_many_lines)]
fn instantiate_service_provider(
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    ops: &mut dyn CallbackOps,
    iter_ref: u64,
) -> Result<u64> {
    let (index, count) = service_iterator_index_and_count(heap, iter_ref)?;
    if index >= count {
        return Err(Error::JavaException {
            class_name: "java/util/NoSuchElementException".to_string(),
        });
    }
    let (loader_ref, provider_name_ref) = {
        let iter = heap.get(iter_ref)?;
        let loader_ref = match iter.fields.get(SERVICE_ITER_LOADER_FIELD) {
            Some(Slot::Reference(reference)) => *reference,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "Reference",
                    got: "other",
                });
            }
        };
        let provider_name_ref = match iter
            .fields
            .get(SERVICE_ITER_PROVIDERS_START + index)
        {
            Some(Slot::Reference(Some(reference))) => *reference,
            _ => return Err(Error::NullPointerException),
        };
        (loader_ref, provider_name_ref)
    };
    let provider_binary_name = string_value_from_ref(heap, provider_name_ref)?;
    let provider_internal_name = binary_name_to_internal_name(&provider_binary_name);
    heap.get_mut(iter_ref)?.fields[SERVICE_ITER_INDEX_FIELD] = Slot::Int(
        i32::try_from(index + 1).unwrap_or(i32::MAX),
    );
    let load_result = if let Some(loader_ref) = loader_ref {
        ops.ensure_loaded_with_runtime_loader(heap, loader_ref, &provider_internal_name)
    } else {
        ops.ensure_loaded(&provider_internal_name)
    };
    if let Err(err) = load_result {
        return Err(
            service_configuration_error(
                &provider_binary_name,
                service_loader_cause_type(&err),
            ),
        );
    }
    let class_key = if let Some(loader_ref) = loader_ref {
        match ops.class_key_for_runtime_loader(heap, loader_ref, &provider_internal_name)
        {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(
                    service_configuration_error(
                        &provider_binary_name,
                        service_loader_cause_type(&err),
                    ),
                );
            }
        }
    } else {
        match ops.class_key_for_loaded_class(&provider_internal_name) {
            Ok(class_key) => class_key,
            Err(err) => {
                return Err(
                    service_configuration_error(
                        &provider_binary_name,
                        service_loader_cause_type(&err),
                    ),
                );
            }
        }
    };
    let reflected = match ops.inspect_class(&class_key) {
        Ok(reflected) => reflected,
        Err(err) => {
            return Err(
                service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ),
            );
        }
    };
    let has_public_no_arg_ctor = reflected
        .methods
        .iter()
        .any(|method| {
            method.name == "<init>" && method.descriptor == "()V" && method.is_public
        });
    if !has_public_no_arg_ctor {
        return Err(
            service_configuration_error(
                &provider_binary_name,
                "java.lang.NoSuchMethodException",
            ),
        );
    }
    let instance_ref = match ops.allocate_instance(heap, output, &class_key) {
        Ok(instance_ref) => instance_ref,
        Err(err) => {
            return Err(
                service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ),
            );
        }
    };
    match ops
        .invoke(
            heap,
            output,
            &class_key,
            "<init>",
            "()V",
            vec![Slot::Reference(Some(instance_ref))],
        )
    {
        Ok(_) => Ok(instance_ref),
        Err(err) => {
            Err(
                service_configuration_error(
                    &provider_binary_name,
                    service_loader_cause_type(&err),
                ),
            )
        }
    }
}
/// Native: `NegatedPredicate.test(O)Z` — inverts the wrapped predicate.
pub(crate) fn native_negated_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let original = extract_first_field_arg(heap, this_ref)?;
    let result = invoke_predicate_test(original, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(!result))))
}
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
        roots.extend(ctx.static_fields.iter().copied());
    }
    roots
}
fn index_out_of_bounds_error() -> Error {
    Error::JavaException {
        class_name: "java/lang/IndexOutOfBoundsException".to_string(),
    }
}
fn timeunit_nanos_per_unit(heap: &duke_gc::Heap, unit_ref: u64) -> Result<i64> {
    match heap.get(unit_ref)?.fields.get(TIMEUNIT_NANOS_FIELD) {
        Some(Slot::Long(nanos)) => Ok(*nanos),
        _ => {
            Err(Error::TypeMismatch {
                expected: "TimeUnit",
                got: "other",
            })
        }
    }
}
pub(crate) fn native_timeunit_to_nanos(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unit_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    Ok(
        Some(
            Slot::Long(
                saturating_mul_i64(value, timeunit_nanos_per_unit(heap, unit_ref)?),
            ),
        ),
    )
}
pub(crate) fn native_timeunit_to_millis(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unit_ref = extract_ref_arg(args, 0)?;
    let value = extract_long_arg(args, 1)?;
    Ok(
        Some(
            Slot::Long(
                saturating_mul_i64(value, timeunit_nanos_per_unit(heap, unit_ref)?)
                    / 1_000_000,
            ),
        ),
    )
}
fn array_element_descriptor(array_descriptor: &str) -> &str {
    array_descriptor.strip_prefix('[').unwrap_or("Ljava/lang/Object;")
}
/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
pub(crate) fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let list_ref = extract_ref_arg(args, 0)?;
    let comparator = args.get(1).copied();
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => {
            return Err(Error::NegativeArraySize {
                size: *n,
            });
        }
        _ => return Ok(None),
    };
    if size <= 1 {
        return Ok(None);
    }
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();
    if elems.len() != size {
        return Err(Error::InvalidRef {
            address: list_ref,
        });
    }
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let cmp = if let Some(Slot::Reference(Some(comp_ref))) = comparator {
                let comp_class = heap.get(comp_ref)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &comp_class,
                    "compare",
                    "(Ljava/lang/Object;Ljava/lang/Object;)I",
                    vec![
                        Slot::Reference(Some(comp_ref)), Slot::Reference(Some(receiver)),
                        Slot::Reference(Some(key)),
                    ],
                )?
            } else {
                let class_name = heap.get(receiver)?.class_name.clone();
                ops.invoke(
                    heap,
                    output,
                    &class_name,
                    COMPARE_TO_METHOD,
                    COMPARE_TO_OBJECT_DESC,
                    vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
                )?
            };
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {}
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }
    Ok(None)
}
fn java_string_hash(value: &str) -> i32 {
    let mut hash = 0_i32;
    for ch in value.chars() {
        hash = hash.wrapping_mul(31).wrapping_add(ch as i32);
    }
    hash
}
fn java_byte_slot(byte: u8) -> Slot {
    Slot::Int(i32::from(i8::from_ne_bytes([byte])))
}
fn java_thread_hosts() -> &'static RwLock<HashMap<i32, std::thread::ThreadId>> {
    static HOSTS: OnceLock<RwLock<HashMap<i32, std::thread::ThreadId>>> = OnceLock::new();
    HOSTS.get_or_init(|| RwLock::new(HashMap::new()))
}
fn java_host_key_for_current_host() -> Option<i32> {
    let host_thread_id = current_host_thread_id();
    java_thread_hosts()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find_map(|(host_key, mapped_host)| {
            (*mapped_host == host_thread_id).then_some(*host_key)
        })
}
/// Maps a `std::cmp::Ordering` to the Java `compareTo` convention: -1 / 0 / 1.
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
const fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
/// Native: `LinkedHashMap.forEach(BiConsumer)V`
pub(crate) fn native_linkedhashmap_for_each(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    native_hashmap_for_each(args, heap, out, control, ops)
}
fn shift_year_month_day(
    year: i32,
    month: u32,
    day: u32,
    delta_months: i64,
) -> (i32, u32, u32) {
    let total_months = i64::from(year) * 12 + (i64::from(month) - 1) + delta_months;
    let new_year = clamp_i64_to_i32(total_months.div_euclid(12));
    let new_month = u32::try_from(total_months.rem_euclid(12) + 1).unwrap_or(1);
    let new_day = day.min(days_in_month(new_year, new_month));
    (new_year, new_month, new_day)
}
fn compute_message_digest(algorithm: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    use sha1::Digest as _;
    let digest = match require_digest_algorithm(algorithm)? {
        "SHA-256" => sha2::Sha256::digest(bytes).to_vec(),
        "SHA-1" => sha1::Sha1::digest(bytes).to_vec(),
        "MD5" => md5::Md5::digest(bytes).to_vec(),
        _ => unreachable!("canonical digest algorithm must be supported"),
    };
    Ok(digest)
}
#[allow(clippy::too_many_arguments)]
fn activate_method_state(
    frame: &mut Frame,
    method_idx: &mut usize,
    pc_to_idx: &mut std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: &mut std::sync::Arc<[(usize, Instruction)]>,
    current_class: &mut String,
    call_stack: &mut Vec<CallFrame>,
    callee_class: String,
    callee_idx: usize,
    callee_pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    callee_frame: Frame,
    callee_instructions: std::sync::Arc<[(usize, Instruction)]>,
    resume_idx: usize,
    #[cfg(feature = "telemetry")]
    registry: &ClassRegistry,
    #[cfg(feature = "telemetry")]
    current_method: &mut String,
) -> Result<()> {
    if call_stack.len() >= MAX_CALL_DEPTH {
        return Err(duke_runtime::Error::JavaException {
            class_name: "java/lang/StackOverflowError".to_string(),
        });
    }
    call_stack
        .push(CallFrame {
            frame: std::mem::replace(frame, callee_frame),
            method_idx: *method_idx,
            pc_to_idx: std::sync::Arc::clone(pc_to_idx),
            resume_idx,
            class_name: current_class.clone(),
        });
    *method_idx = callee_idx;
    *pc_to_idx = callee_pc_to_idx;
    *current_class = callee_class;
    *instructions = callee_instructions;
    #[cfg(feature = "telemetry")]
    refresh_current_method_name(current_method, registry, current_class, *method_idx);
    Ok(())
}
fn deadline_from_now(nanos: i64) -> Option<std::time::Instant> {
    let now = std::time::Instant::now();
    u64::try_from(nanos)
        .ok()
        .and_then(|nanos| now.checked_add(std::time::Duration::from_nanos(nanos)))
}
/// Native: `ThenComparingComparator.compare(O,O)I` — runs primary then secondary.
pub(crate) fn native_then_comparing_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let a = extract_slot_arg(args, 1);
    let b = extract_slot_arg(args, 2);
    let primary = extract_first_field_arg(heap, this_ref)?;
    let secondary = extract_field_arg(heap, this_ref, 1)?;
    let result = invoke_comparator(primary, a, b, heap, out, ops)?;
    if result != 0 {
        return Ok(Some(Slot::Int(result)));
    }
    let result2 = invoke_comparator(secondary, a, b, heap, out, ops)?;
    Ok(Some(Slot::Int(result2)))
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_posix_file_permissions_as_file_attribute(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let attribute_ref = heap
        .allocate("java/nio/file/attribute/FileAttribute".to_string(), 0);
    Ok(Some(Slot::Reference(Some(attribute_ref))))
}
fn alloc_byte_array(heap: &mut duke_gc::Heap, bytes: &[u8]) -> u64 {
    let array_ref = heap.allocate("[B".to_string(), bytes.len());
    if let Ok(array) = heap.get_mut(array_ref) {
        for (idx, byte) in bytes.iter().copied().enumerate() {
            array.fields[idx] = Slot::Int(i32::from(byte));
        }
    }
    array_ref
}
fn alloc_string_array_from_parts(
    heap: &mut duke_gc::Heap,
    parts: &[String],
) -> Result<u64> {
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[idx] = Slot::Reference(Some(str_ref));
    }
    Ok(arr_ref)
}
pub(crate) fn native_float_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Float(val);
    Ok(Some(Slot::Reference(Some(r))))
}
pub(crate) fn native_float_floatvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}
pub(crate) fn native_float_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let float_val = |s: &Slot| -> Result<f32> {
        match s {
            Slot::Reference(Some(r)) => {
                match heap.get(*r)?.fields.first() {
                    Some(Slot::Float(n)) => Ok(*n),
                    _ => Err(Error::InvalidRef { address: *r }),
                }
            }
            _ => Err(Error::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => float_val(s)?,
        None => return Err(Error::NullPointerException),
    };
    let b = float_val(args.get(1).ok_or(Error::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}
/// Native: `Float.parseFloat(String)` — parses string to float.
pub(crate) fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(Error::NullPointerException),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s
        .trim()
        .parse()
        .map_err(|_| Error::JavaException {
            class_name: "java/lang/NumberFormatException".to_string(),
        })?;
    Ok(Some(Slot::Float(val)))
}
fn pending_java_exception_messages() -> &'static PendingExceptionMessages {
    static MESSAGES: OnceLock<PendingExceptionMessages> = OnceLock::new();
    MESSAGES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}
fn pending_java_exception_causes() -> &'static PendingExceptionCauses {
    static CAUSES: OnceLock<PendingExceptionCauses> = OnceLock::new();
    CAUSES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}
fn clamp_i128_to_i64(value: i128) -> i64 {
    match i64::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}
fn clamp_i64_to_i32(value: i64) -> i32 {
    match i32::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => i32::MIN,
        Err(_) => i32::MAX,
    }
}
#[allow(clippy::too_many_lines)]
fn format_arg(
    spec: char,
    flags: &str,
    width: Option<usize>,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> Result<String> {
    let left_align = flags.contains('-');
    let force_sign = flags.contains('+');
    let zero_pad = flags.contains('0') && !left_align;
    let raw = match slot {
        Slot::Reference(None) => {
            if spec == 'b' { "false".to_string() } else { "null".to_string() }
        }
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => heap_object_to_string(obj, *r),
                'b' => {
                    if obj.class_name == "java/lang/Boolean" {
                        match obj.fields.first() {
                            Some(Slot::Int(n)) => {
                                if *n != 0 { "true" } else { "false" }.to_string()
                            }
                            _ => "true".to_string(),
                        }
                    } else {
                        "true".to_string()
                    }
                }
                'c' => {
                    let code = match obj.fields.first() {
                        Some(Slot::Int(n)) => *n,
                        _ => 0,
                    };
                    #[allow(clippy::cast_sign_loss)]
                    char::from_u32(code as u32).map_or(String::new(), |c| c.to_string())
                }
                'd' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Int(v)) => i64::from(*v),
                        Some(Slot::Long(v)) => *v,
                        _ => 0,
                    };
                    if force_sign && v >= 0 { format!("+{v}") } else { v.to_string() }
                }
                'o' => {
                    match obj.fields.first() {
                        Some(Slot::Int(v)) => format!("{v:o}"),
                        Some(Slot::Long(v)) => format!("{v:o}"),
                        _ => "0".to_string(),
                    }
                }
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let s = precision
                        .map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                    if force_sign && v >= 0.0 { format!("+{s}") } else { s }
                }
                'e' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    let prec = precision.unwrap_or(6);
                    let s = format_scientific(v, prec, false);
                    if force_sign && v >= 0.0 { format!("+{s}") } else { s }
                }
                'x' => {
                    match obj.fields.first() {
                        Some(Slot::Int(v)) => format!("{v:x}"),
                        Some(Slot::Long(v)) => format!("{v:x}"),
                        _ => "0".to_string(),
                    }
                }
                'X' => {
                    match obj.fields.first() {
                        Some(Slot::Int(v)) => format!("{v:X}"),
                        Some(Slot::Long(v)) => format!("{v:X}"),
                        _ => "0".to_string(),
                    }
                }
                _ => String::new(),
            }
        }
        Slot::Int(n) => {
            match spec {
                'd' => {
                    if force_sign && *n >= 0 { format!("+{n}") } else { n.to_string() }
                }
                'b' => "true".to_string(),
                'c' => {
                    #[allow(clippy::cast_sign_loss)]
                    char::from_u32(*n as u32).map_or(String::new(), |c| c.to_string())
                }
                'o' => format!("{n:o}"),
                'x' => format!("{n:x}"),
                'X' => format!("{n:X}"),
                _ => n.to_string(),
            }
        }
        Slot::Long(n) => {
            match spec {
                'd' => {
                    if force_sign && *n >= 0 { format!("+{n}") } else { n.to_string() }
                }
                'o' => format!("{n:o}"),
                'x' => format!("{n:x}"),
                'X' => format!("{n:X}"),
                _ => n.to_string(),
            }
        }
        Slot::Double(v) => {
            match spec {
                'f' => {
                    let s = precision
                        .map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}"));
                    if force_sign && *v >= 0.0 { format!("+{s}") } else { s }
                }
                'e' => {
                    let prec = precision.unwrap_or(6);
                    let s = format_scientific(*v, prec, false);
                    if force_sign && *v >= 0.0 { format!("+{s}") } else { s }
                }
                _ => format!("{v}"),
            }
        }
        _ => String::new(),
    };
    Ok(
        match width {
            None => raw,
            Some(w) => apply_format_width(raw, w, left_align, zero_pad),
        },
    )
}
/// Format a float in Java-style scientific notation `1.234568e+05`.
fn format_scientific(v: f64, prec: usize, upper: bool) -> String {
    let max_prec = 1024 * 1024 * 128;
    let prec = prec.min(max_prec);
    if v == 0.0 {
        let zeros = "0".repeat(prec);
        let e = if upper { 'E' } else { 'e' };
        return format!("0.{zeros}{e}+00");
    }
    #[allow(clippy::cast_possible_truncation)]
    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10_f64.powi(exp);
    let s = format!("{mantissa:.prec$}");
    let e_char = if upper { 'E' } else { 'e' };
    if exp >= 0 {
        format!("{s}{e_char}+{exp:02}")
    } else {
        format!("{s}{e_char}-{:02}", exp.unsigned_abs())
    }
}
/// Format a float like Java's Float.toString.
fn format_java_float(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}
/// Format a double like Java's Double.toString.
fn format_java_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}
fn patch_forwarded_slot_if_needed(heap: &duke_gc::Heap, slot: &mut Slot) {
    if heap.has_pending_forwards() {
        heap.apply_forward(slot);
    }
}
fn patch_forwarded_ref_if_needed(heap: &duke_gc::Heap, reference: &mut u64) {
    let mut slot = Slot::Reference(Some(*reference));
    patch_forwarded_slot_if_needed(heap, &mut slot);
    if let Slot::Reference(Some(new_ref)) = slot {
        *reference = new_ref;
    }
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
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
}
/// Native: `ReverseOrderComparator.compare(O,O)I` — negates natural order.
pub(crate) fn native_reverse_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let result = native_natural_order_compare(args, heap, out, control, ops)?;
    Ok(
        Some(
            match result {
                Some(Slot::Int(v)) => Slot::Int(-v),
                other => other.unwrap_or(Slot::Int(0)),
            },
        ),
    )
}
/// Formats a single boxed slot value using the given format specifier.
/// Apply width/alignment/flags to an already-formatted value string.
fn apply_format_width(
    s: String,
    width: usize,
    left_align: bool,
    zero_pad: bool,
) -> String {
    let max_width = 1024 * 1024 * 128;
    let width = width.min(max_width);
    if s.len() >= width {
        return s;
    }
    let pad = width - s.len();
    if left_align {
        format!("{s}{}", " ".repeat(pad))
    } else if zero_pad {
        if s.starts_with('-') || s.starts_with('+') {
            let (sign, rest) = s.split_at(1);
            format!("{sign}{}{rest}", "0".repeat(pad))
        } else {
            format!("{}{s}", "0".repeat(pad))
        }
    } else {
        format!("{}{s}", " ".repeat(pad))
    }
}
/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> Result<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => {
            Err(Error::InvalidCpIndex {
                index: idx,
            })
        }
    }
}
/// Execute a `StringConcatFactory` recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    arg_types: &[char],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> Result<Slot> {
    let mut result = String::new();
    let mut dyn_idx = 0;
    let mut const_idx = 0;
    for ch in recipe.chars() {
        match ch {
            '\u{1}' => {
                if dyn_idx < dynamic_args.len() {
                    let type_hint = arg_types.get(dyn_idx).copied().unwrap_or('I');
                    stringify_slot(
                        &dynamic_args[dyn_idx],
                        type_hint,
                        heap,
                        &mut result,
                    )?;
                    dyn_idx += 1;
                }
            }
            '\u{2}' => {
                if const_idx < constants.len() {
                    result.push_str(&constants[const_idx]);
                    const_idx += 1;
                }
            }
            other => result.push(other),
        }
    }
    let r = heap.allocate_string(result);
    Ok(Slot::Reference(Some(r)))
}
/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack` from the class file
/// - `max_locals`: `Code.max_locals` from the class file
///
/// # Returns
/// `Ok(Some(slot))` for value-returning methods, `Ok(None)` for `void`.
///
/// # Errors
/// Returns [`Error`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Panics
///
/// Contains internal `debug_assert!` checks that will panic in debug mode if the JVM
/// state becomes invalid (e.g., stack underflow).
///
/// # Examples
///
/// The core bytecode interpretation loop.
///
/// **Why it exists:** This is the execution engine of the JVM. It reads decoded instructions,
/// manipulates the operand stack and local variables, and manages control flow (jumps, branches).
///
/// # Arguments
///
/// * `instructions` - A slice of `(pc, Instruction)` pairs representing the method's bytecode.
/// * `cp` - The constant pool for the current class.
/// * `args` - The initial local variables (arguments to the method).
/// * `max_stack` - The maximum depth of the operand stack.
/// * `max_locals` - The size of the local variable array.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes via `ireturn`, `areturn`, etc., yielding a value.
/// * `Ok(None)` - If the method completes via `return` (void).
/// * `Err(Error)` - If an exception is thrown or a fatal error occurs.
///
/// # Examples
///
/// Executes a sequence of instructions (bytecode) independently.
///
/// This is used heavily internally by the `MethodHandle` resolution and native implementation
/// logic to run standalone bytecodes (e.g., dynamically generated stubs).
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_runtime::Slot;
/// use duke_interpreter::execute;
///
/// let instructions = vec![
///     (0, Instruction::Iconst5),
///     (1, Instruction::Ireturn),
/// ];
/// let cp = vec![];
/// let result = execute(&instructions, &cp, vec![], 1, 0).unwrap();
/// assert_eq!(result, Some(Slot::Int(5)));
/// ```
///
/// # Errors
///
/// Returns a `Error` if bytecode invariants are broken or if an exception is raised
/// internally.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> Result<Option<Slot>> {
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();
    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<(u8, String), u64> = HashMap::new();
    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(Error::FellOffEnd);
        };
        let pc = *pc;
        macro_rules! jump {
            ($offset:expr) => {
                { let target = (pc as i64).wrapping_add(i64::from($offset)) as usize; idx
                = * pc_to_idx.get(& target).ok_or(Error::InvalidBranchTarget { pc :
                target }) ?; continue; }
            };
        }
        match instr {
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
                if let Some(CpEntry::String { string_index }) = cp
                    .get(cp_idx)
                    .and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let s = match cp.get(si).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidCpIndex { index: si }),
                    };
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = local_heap.len() as u64;
                        local_heap
                            .push((
                                "java/lang/String".to_string(),
                                Vec::new(),
                                Some(intern_key.1.clone()),
                            ));
                        string_intern.insert(intern_key, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) = cp
                    .get(idx_val)
                    .and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let s = match cp.get(si).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(Error::InvalidCpIndex { index: si }),
                    };
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = local_heap.len() as u64;
                        local_heap
                            .push((
                                "java/lang/String".to_string(),
                                Vec::new(),
                                Some(intern_key.1.clone()),
                            ));
                        string_intern.insert(intern_key, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
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
                let value = frame.pop()?;
                if !matches!(value, Slot::Long(_) | Slot::Double(_)) {
                    frame.pop()?;
                }
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
                    return Err(Error::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(Error::DivisionByZero);
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
                frame
                    .store_local(
                        usize::from(*index),
                        Slot::Int(v.wrapping_add(i32::from(*value))),
                    )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame
                    .store_local(
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
                    return Err(Error::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(Error::DivisionByZero);
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
                #[allow(clippy::float_cmp)]
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
                #[allow(clippy::float_cmp)]
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
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),
            Instruction::Goto(offset) => jump!(* offset),
            Instruction::GotoW(offset) => jump!(* offset),
            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(* offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(* offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(* offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(* offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(* offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(* offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop() ?, Slot::Reference(None)) {
                    jump!(* offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop() ?, Slot::Reference(None)) {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(* offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(* offset);
                }
            }
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(* offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(* offset);
                }
            }
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(Error::NegativeArraySize {
                        size: count,
                    });
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
                local_heap
                    .push((
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
                    return Err(Error::NegativeArraySize {
                        size: count,
                    });
                }
                let r = local_heap.len() as u64;
                local_heap
                    .push((
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
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
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i8);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as u16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(Error::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }
            Instruction::Tableswitch { default, low, high, offsets } => {
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
                            .ok_or(Error::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(Error::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(Error::TypeMismatch {
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
                            .ok_or(Error::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(Error::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(Error::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(Error::JavaException { class_name });
            }
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }
            other => {
                return Err(Error::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }
        idx += 1;
    }
}
/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`Error`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
///
/// # Examples
///
/// ```
/// use std::io::sink;
/// use duke_runtime::Slot;
/// use duke_gc::Heap;
/// use duke_loader::DirectoryLoader;
/// use duke_interpreter::{execute_class, ClassRegistry};
///
/// let mut registry = ClassRegistry::new();
/// let loader = DirectoryLoader::new("my_classes");
/// let mut heap = Heap::new();
/// let mut out = sink();
///
/// // We simulate execution by calling execute_class.
/// // It fails here with a missing class error, but demonstrates the setup.
/// let res = execute_class(&mut registry, &loader, &mut heap, &mut out, "java/lang/Object", "hashCode", "()I", &[]);
/// assert!(res.is_err());
/// ```
/// Starts execution of a specific Java method within a given class.
///
/// **Why it exists:** This function resolves the requested class and method, builds the
/// initial execution frame, and begins interpreting instructions. It bridges the gap
/// between the user requesting a class to run and the `execute` loop.
///
/// # Arguments
/// * `registry` - The class registry for resolving types.
/// * `loader` - The class loader for fetching dependencies.
/// * `heap` - The garbage collector heap.
/// * `stdout` - The standard output stream.
/// * `class_name` - The internal name of the class (e.g., `"java/lang/String"`).
/// * `method_name` - The name of the method to execute (e.g., `"main"`).
/// * `descriptor` - The method descriptor (e.g., `"([Ljava/lang/String;)V"`).
/// * `args` - The arguments to pass to the method.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes and returns a value.
/// * `Ok(None)` - If the method is `void` and completes.
/// * `Err(Error)` - If the method throws an unhandled exception or encounters a fatal VM error.
///
/// # Examples
///
/// ```no_run
/// # use duke_interpreter::{execute_class, ClassRegistry};
/// # use duke_loader::DirectoryLoader;
/// # use duke_gc::Heap;
/// # let mut registry = ClassRegistry::new();
/// # let loader = DirectoryLoader::new(".");
/// # let mut heap = Heap::new();
/// # let mut stdout = Vec::new();
/// // Execute `public static void main(String[] args)`
/// let result = execute_class(
///     &mut registry,
///     &loader,
///     &mut heap,
///     &mut stdout,
///     "com/example/App",
///     "main",
///     "([Ljava/lang/String;)V",
///     &[]
/// );
/// ```
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
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
) -> Result<Option<Slot>> {
    match lookup_registered_native_kind(registry, class_name, method_name, descriptor) {
        Some(HandlerKind::Simple(h)) => {
            let mut native_control = NativeControl::default();
            let result = h(args, heap, stdout, &mut native_control)?;
            if native_control.take().is_some() {
                return Err(Error::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        Some(HandlerKind::Callback(h)) => {
            let mut native_control = NativeControl::default();
            let result = {
                let mut callback_ops = InterpreterCallbackOps {
                    registry,
                    loader,
                };
                h(args, heap, stdout, &mut native_control, &mut callback_ops)?
            };
            if native_control.take().is_some() {
                return Err(Error::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        None => {}
    }
    let mut state = prepare_execution_state(
        registry,
        loader,
        heap,
        stdout,
        class_name,
        method_name,
        descriptor,
        args,
    )?;
    match execution::run_execution(
        &mut state,
        registry,
        loader,
        heap,
        stdout,
        true,
        None,
    )? {
        ExecutionOutcome::Returned(result) => Ok(result),
        ExecutionOutcome::ThreadAction(_) | ExecutionOutcome::Yield => {
            Err(Error::Unimplemented {
                mnemonic: "thread action requires execute_class_to_completion",
            })
        }
    }
}
/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `Error` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
/// Starts execution of a specific Java method within a given class.
///
/// **Why it exists:** This function resolves the requested class and method, builds the
/// initial execution frame, and begins interpreting instructions. It bridges the gap
/// between the user requesting a class to run and the `execute` loop.
///
/// # Arguments
/// * `registry` - The class registry for resolving types.
/// * `loader` - The class loader for fetching dependencies.
/// * `heap` - The garbage collector heap.
/// * `stdout` - The standard output stream.
/// * `class_name` - The internal name of the class (e.g., `"java/lang/String"`).
/// * `method_name` - The name of the method to execute (e.g., `"main"`).
/// * `descriptor` - The method descriptor (e.g., `"([Ljava/lang/String;)V"`).
/// * `args` - The arguments to pass to the method.
///
/// # Returns
///
/// * `Ok(Some(Slot))` - If the method completes and returns a value.
/// * `Ok(None)` - If the method is `void` and completes.
/// * `Err(Error)` - If the method throws an unhandled exception or encounters a fatal VM error.
///
/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `Error` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
///
/// # Panics
///
/// Panics if a `Mutex` protecting shared VM state is poisoned by a
/// panicking thread, or if the `Arc` cannot be unwound after all threads
/// have joined. Also panics if the completion runtime is unexpectedly released early.
#[allow(clippy::too_many_arguments, clippy::missing_panics_doc)]
pub fn execute_class_to_completion<L>(
    registry: &mut ClassRegistry,
    loader: L,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> Result<Option<Slot>>
where
    L: ClassLoader + Send + Sync + 'static,
{
    let loader: std::sync::Arc<dyn ClassLoader + Send + Sync> = std::sync::Arc::new(
        loader,
    );
    if lookup_registered_native_kind(registry, class_name, method_name, descriptor)
        .is_some()
    {
        return execute_class(
            registry,
            loader.as_ref(),
            heap,
            stdout,
            class_name,
            method_name,
            descriptor,
            args,
        );
    }
    let mut vm = CompletionVm {
        registry: std::mem::take(registry),
        heap: std::mem::replace(heap, duke_gc::Heap::new()),
        output: Vec::new(),
        live_workers: 0,
    };
    let mut state = match prepare_execution_state(
        &mut vm.registry,
        loader.as_ref(),
        &mut vm.heap,
        &mut vm.output,
        class_name,
        method_name,
        descriptor,
        args,
    ) {
        Ok(state) => state,
        Err(err) => {
            *registry = vm.registry;
            *heap = vm.heap;
            return Err(err);
        }
    };
    let shared = std::sync::Arc::new(std::sync::Mutex::new(vm));
    let runtime = std::sync::Arc::new(
        std::sync::Mutex::new(CompletionRuntime::default()),
    );
    let run_result: Result<Option<Slot>> = loop {
        let mut shared_guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let CompletionVm { registry, heap, output, live_workers } = &mut *shared_guard;
        let outcome = execution::run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
            Some(DEFAULT_THREAD_QUANTUM),
        )?;
        drop(shared_guard);
        match outcome {
            ExecutionOutcome::Returned(result) => break Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, &shared, &runtime, &loader)?;
            }
            ExecutionOutcome::Yield => {
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
    };
    let wait_result = wait_for_all_java_threads(&runtime);
    let Ok(shared) = std::sync::Arc::try_unwrap(shared) else {
        panic!("completion runtime released shared VM state")
    };
    let Ok(shared) = shared.into_inner() else { panic!("shared VM mutex poisoned") };
    let flush_result = stdout
        .write_all(&shared.output)
        .map_err(|_| Error::Unimplemented {
            mnemonic: "output flush",
        });
    *registry = shared.registry;
    *heap = shared.heap;
    match run_result {
        Ok(result) => {
            wait_result?;
            flush_result?;
            Ok(result)
        }
        Err(err) => {
            let _ = wait_result;
            let _ = flush_result;
            Err(err)
        }
    }
}
pub(crate) fn native_attributes_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let attributes_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap
        .get(attributes_ref)?
        .fields
        .first()
        .copied() else {
        return Err(Error::NullPointerException);
    };
    let manifest_text = string_value_from_ref(heap, raw_ref)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = manifest_attribute_value(manifest_text.as_bytes(), &key)
        .map_or(
            Slot::Reference(None),
            |value| { Slot::Reference(Some(heap.allocate_string(value))) },
        );
    Ok(Some(result))
}
/// Autobox a primitive return value when the SAM descriptor expects a reference.
///
/// This is needed for bound method references like `s::length` where the impl
/// returns `int` but the SAM interface (`Supplier.get()`) returns `Object`.
fn autobox_if_needed(
    result: Option<Slot>,
    impl_desc: &str,
    sam_desc: &str,
    heap: &mut duke_gc::Heap,
) -> Result<Option<Slot>> {
    let Some(slot) = result else {
        return Ok(None);
    };
    if !matches!(desc_return_char(sam_desc), Some('L' | '[')) {
        return Ok(Some(slot));
    }
    match (desc_return_char(impl_desc), slot) {
        (Some('I'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('Z'), Slot::Int(v)) => {
            let r = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Int(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('J'), Slot::Long(v)) => {
            let r = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Long(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('D'), Slot::Double(v)) => {
            let r = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Double(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        (Some('F'), Slot::Float(v)) => {
            let r = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(r)?.fields[0] = Slot::Float(v);
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Ok(Some(slot)),
    }
}
/// Expand method arguments to JVM local variable slot layout.
///
/// In the JVM spec, `long` (`J`) and `double` (`D`) parameters each occupy
/// *two* local variable slots. The second slot is a phantom placeholder so
/// that all subsequent parameters are addressed at the correct index.
/// For example, a static `(JJ)J` method uses `lload_0` / `lload_2`; without
/// expansion, Duke would pack both longs at indices 0 and 1, causing
/// `lload_2` to see `Slot::Int(0)`.
///
/// If the descriptor contains no wide types this is a zero-copy clone.
/// Adapts `args` to match the parameter types in `descriptor`:
/// - Inserts a padding `Slot::Int(0)` after each Long/Double slot.
/// - Unboxes `Slot::Reference(Some(r))` → `Slot::Int/Long/Float/Double` when
///   the corresponding descriptor param type is `I`, `J`, `F`, or `D`.
///   This handles method references like `Integer::sum` used as `BinaryOperator<Integer>`.
fn adapt_args_for_impl_desc(
    args: &[Slot],
    descriptor: &str,
    heap: &duke_gc::Heap,
) -> Vec<Slot> {
    let param_types = parse_arg_types(descriptor);
    if param_types.is_empty() || args.is_empty() {
        return args.to_vec();
    }
    let mut result = Vec::with_capacity(args.len() + 4);
    for (slot, &type_char) in args.iter().zip(param_types.iter()) {
        let adapted = match (slot, type_char) {
            (Slot::Reference(Some(r)), 'I' | 'B' | 'S' | 'C' | 'Z') => {
                heap.get(*r)
                    .ok()
                    .and_then(|obj| obj.fields.first().copied())
                    .filter(|s| matches!(s, Slot::Int(_)))
                    .unwrap_or(*slot)
            }
            (Slot::Reference(Some(r)), 'J') => {
                heap.get(*r)
                    .ok()
                    .and_then(|obj| obj.fields.first().copied())
                    .filter(|s| matches!(s, Slot::Long(_)))
                    .unwrap_or(*slot)
            }
            (Slot::Reference(Some(r)), 'D') => {
                heap.get(*r)
                    .ok()
                    .and_then(|obj| obj.fields.first().copied())
                    .filter(|s| matches!(s, Slot::Double(_)))
                    .unwrap_or(*slot)
            }
            (Slot::Reference(Some(r)), 'F') => {
                heap.get(*r)
                    .ok()
                    .and_then(|obj| obj.fields.first().copied())
                    .filter(|s| matches!(s, Slot::Float(_)))
                    .unwrap_or(*slot)
            }
            _ => *slot,
        };
        result.push(adapted);
        if type_char == 'J' || type_char == 'D' {
            result.push(Slot::Int(0));
        }
    }
    if args.len() > param_types.len() {
        result.extend_from_slice(&args[param_types.len()..]);
    }
    result
}
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_void_noop(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}
fn unescape_property_text(raw: &str) -> Result<String> {
    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        let Some(escaped) = chars.next() else {
            result.push('\\');
            break;
        };
        match escaped {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'f' => result.push('\u{000c}'),
            'u' => {
                let mut code = 0_u32;
                for _ in 0..4 {
                    let Some(hex) = chars.next().and_then(hex_value) else {
                        return Err(properties_illegal_argument());
                    };
                    code = (code << 4) | hex;
                }
                let Some(decoded) = char::from_u32(code) else {
                    return Err(properties_illegal_argument());
                };
                result.push(decoded);
            }
            other => result.push(other),
        }
    }
    Ok(result)
}
/// Convert a proleptic Gregorian epoch day to (year, month, day).
#[allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_const_for_fn
)]
fn epoch_days_to_ymd(epoch_days: i32) -> (i32, u32, u32) {
    let z = epoch_days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let yoe = (day_of_era - day_of_era / 1460 + day_of_era / 36524
        - day_of_era / 146_096) / 365;
    let y = yoe + era * 400;
    let day_of_year = day_of_era - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let d = day_of_year - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}
fn epoch_seconds_to_datetime_parts(
    epoch_seconds: i64,
) -> (i32, u32, u32, i32, i32, i32) {
    let epoch_days = clamp_i64_to_i32(epoch_seconds.div_euclid(SECONDS_PER_DAY_I64));
    let second_of_day = epoch_seconds.rem_euclid(SECONDS_PER_DAY_I64);
    let hour = i32::try_from(second_of_day / 3600).unwrap_or(0);
    let minute = i32::try_from((second_of_day % 3600) / 60).unwrap_or(0);
    let second = i32::try_from(second_of_day % 60).unwrap_or(0);
    let (year, month, day) = epoch_days_to_ymd(epoch_days);
    (year, month, day, hour, minute, second)
}
fn escape_property_key(key: &str) -> String {
    let mut result = String::new();
    for ch in key.chars() {
        push_escaped_key_char(&mut result, ch);
    }
    result
}
fn escape_property_value(value: &str) -> String {
    let mut result = String::new();
    for (idx, ch) in value.chars().enumerate() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{000c}' => result.push_str("\\f"),
            ' ' if idx == 0 => result.push_str("\\ "),
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}
fn escape_property_comment(comment: &str) -> String {
    let mut result = String::new();
    for ch in comment.chars() {
        match ch {
            '\n' | '\r' => {}
            _ if !ch.is_ascii() || u32::from(ch) < 0x20 || u32::from(ch) > 0x7e => {
                push_unicode_escape(&mut result, ch);
            }
            _ => result.push(ch),
        }
    }
    result
}
/// Convert (year, month, day) to a proleptic Gregorian epoch day count.
/// Day 0 = 1970-01-01.  Howard Hinnant's branchless algorithm.
#[allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn
)]
fn ymd_to_epoch_days(year: i32, month: u32, day: u32) -> i32 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let day_of_year = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + day_of_year;
    (era * 146_097 + doe - 719_468) as i32
}
/// Native: `OrPredicate.test(O)Z` — either predicate returning true is sufficient.
pub(crate) fn native_or_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = extract_slot_arg(args, 1);
    let left = extract_first_field_arg(heap, this_ref)?;
    let right = extract_field_arg(heap, this_ref, 1)?;
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if la {
        return Ok(Some(Slot::Int(1)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}
fn finish_native_call(
    native_control: &mut NativeControl,
    frame: &mut Frame,
    idx: &mut usize,
    result: Option<Slot>,
    retry_args: &[Slot],
) -> Result<Option<ExecutionOutcome>> {
    let action = native_control.take();
    if matches!(action, Some(NativeThreadAction::Retry)) {
        for slot in retry_args {
            frame.push(*slot)?;
        }
        return Ok(Some(ExecutionOutcome::ThreadAction(NativeThreadAction::Retry)));
    }
    if let Some(val) = result {
        frame.push(val)?;
    }
    *idx += 1;
    if let Some(action) = action {
        return Ok(Some(ExecutionOutcome::ThreadAction(action)));
    }
    Ok(None)
}
fn find_annotation<'a>(
    annotations: &'a [ReflectedAnnotation],
    requested_type: &str,
) -> Option<&'a ReflectedAnnotation> {
    let requested_type = class_internal_name_from_key(requested_type);
    annotations.iter().find(|annotation| annotation.type_name == requested_type)
}
/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    handler_class: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table
        .iter()
        .find_map(|entry| {
            let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
            #[allow(clippy::option_if_let_else)]
            let type_matches = match &entry.catch_type {
                None => true,
                Some(ct) => {
                    is_assignable_from(
                        registry,
                        loader,
                        class_name,
                        ct,
                        Some(handler_class),
                    )
                }
            };
            if in_range && type_matches { Some(entry.handler_pc) } else { None }
        })
}
/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(
    fields: &[Slot],
    key: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
}
fn find_hashset_entry_index(
    fields: &[Slot],
    element: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    fields
        .iter()
        .skip(1)
        .position(|field| slots_equal(field, element, heap))
        .map(|idx| idx + 1)
}
fn manifest_attribute_value(manifest_bytes: &[u8], key: &str) -> Option<String> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix(':')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}
pub(crate) fn native_manifest_get_main_attributes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let manifest_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap
        .get(manifest_ref)?
        .fields
        .first()
        .copied() else {
        return Err(Error::NullPointerException);
    };
    let attributes_ref = heap.allocate("java/util/jar/Attributes".to_string(), 1);
    heap.get_mut(attributes_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(Some(Slot::Reference(Some(attributes_ref))))
}
/// Native: `Socket.<init>(String host, int port)` — connects to host:port.
pub(crate) fn native_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let host_ref = extract_ref_arg(args, 1)?;
    let host = heap
        .get(host_ref)?
        .string_value
        .clone()
        .ok_or(Error::NullPointerException)?;
    let port = extract_int_arg(args, 2)?;
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(Error::InvalidRef {
            address: this_ref,
        });
    }
    obj.fields[0] = Slot::Int(reader_id);
    obj.fields[1] = Slot::Int(writer_id);
    Ok(None)
}
/// Native: `Socket.getInputStream()` — allocates a `SocketInputStream` wrapping `fdRead`.
pub(crate) fn native_socket_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketInputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_read);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `Socket.getOutputStream()` — allocates a `SocketOutputStream` wrapping `fdWrite`.
pub(crate) fn native_socket_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(Error::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketOutputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_write);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}
/// Native: `Socket.close()` — closes both OS handles (fdRead and fdWrite).
pub(crate) fn native_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = extract_io_fd(heap, this_ref)?;
    let fd_write = extract_io_fd_at(heap, this_ref, 1)?;
    heap.close_host_file(fd_read);
    heap.close_host_file(fd_write);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}
fn last_match_bounds(heap: &duke_gc::Heap, m_ref: u64) -> Result<(usize, usize)> {
    let start = matcher_field_int(heap, m_ref, MATCHER_MATCH_START_FIELD, -1)?;
    let end = matcher_field_int(heap, m_ref, MATCHER_MATCH_END_FIELD, 0)?;
    if start < 0 {
        return Err(regex_illegal_state("No match available"));
    }
    Ok((usize::try_from(start).unwrap_or(0), usize::try_from(end.max(0)).unwrap_or(0)))
}
fn host_thread_for_java_thread(host_key: i32) -> Option<std::thread::ThreadId> {
    java_thread_hosts()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&host_key)
        .copied()
}
fn uses_first_field_value_equality(class_name: &str) -> bool {
    matches!(
        class_name, "java/lang/Boolean" | "java/lang/Byte" | "java/lang/Character" |
        "java/lang/Double" | "java/lang/Float" | "java/lang/Integer" | "java/lang/Long" |
        "java/lang/Short"
    )
}
fn next_thread_host_key() -> i32 {
    NEXT_THREAD_HOST_KEY.fetch_add(1, Ordering::Relaxed)
}
fn next_find_pos(input: &str, start: usize, end: usize) -> usize {
    if start != end || end >= input.len() {
        return end;
    }
    input[end..].chars().next().map_or(end, |ch| end + ch.len_utf8())
}
fn initialize_primitive_wrapper_type_field(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> Result<()> {
    let Some(descriptor) = primitive_wrapper_type_descriptor(class_name) else {
        return Ok(());
    };
    let type_slot = {
        let ctx = registry.get(class_name)?;
        static_field_idx(ctx, "TYPE")?
    };
    if matches!(
        registry.get(class_name) ?.static_fields.get(type_slot),
        Some(Slot::Reference(Some(_)))
    ) {
        return Ok(());
    }
    let class_ref = allocate_class_object(heap, descriptor)?;
    registry.get_mut(class_name)?.static_fields[type_slot] = Slot::Reference(
        Some(class_ref),
    );
    Ok(())
}
fn translate_java_named_groups(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            let mut probe = chars.clone();
            if probe.next() == Some('?') && probe.next() == Some('<') {
                match probe.next() {
                    Some('=' | '!') | None => out.push(ch),
                    Some(_) => {
                        out.push_str("(?P<");
                        chars.next();
                        chars.next();
                    }
                }
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}
fn log_resource_lookup_miss(resolved_name: &str, base: &str, classpath: &str) {
    if resource_lookup_trace_enabled() {
        eprintln!(
            "resource lookup miss: name={resolved_name} base={base} classpath={classpath}"
        );
    }
}
#[allow(clippy::too_many_arguments, clippy::option_option)]
fn callback_invoke_registered_lambda(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    class: &str,
    method: &str,
    descriptor: &str,
    args: &[Slot],
) -> Result<Option<Option<Slot>>> {
    let Some(lambda_info) = registry.get_lambda(class).cloned() else {
        return Ok(None);
    };
    if method != lambda_info.sam_method || descriptor != lambda_info.sam_desc {
        return Ok(None);
    }
    let this_ref = match args.first().copied() {
        Some(Slot::Reference(Some(reference))) => reference,
        Some(Slot::Reference(None)) | None => return Err(Error::NullPointerException),
        Some(_) => {
            return Err(Error::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let lambda_object = heap.get(this_ref)?;
    let mut impl_args = Vec::with_capacity(
        lambda_info.captured_count + args.len().saturating_sub(1),
    );
    for capture_index in 0..lambda_info.captured_count {
        let slot = lambda_object
            .fields
            .get(capture_index)
            .copied()
            .ok_or(Error::Unimplemented {
                mnemonic: "lambda capture missing",
            })?;
        impl_args.push(slot);
    }
    impl_args.extend_from_slice(&args[1..]);
    let dispatch_class = match lambda_info.impl_kind {
        6 | 7 => lambda_info.impl_class.clone(),
        5 | 9 => {
            match impl_args.first().copied() {
                Some(Slot::Reference(Some(receiver_ref))) => {
                    let receiver_class = heap.get(receiver_ref)?.class_name.clone();
                    if has_registered_native_override(
                        registry,
                        &receiver_class,
                        &lambda_info.impl_method,
                        &lambda_info.impl_desc,
                    ) {
                        receiver_class
                    } else if let Some((dispatch_class, _)) = resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &receiver_class,
                        &lambda_info.impl_method,
                        &lambda_info.impl_desc,
                    ) {
                        dispatch_class
                    } else {
                        lambda_info.impl_class.clone()
                    }
                }
                Some(Slot::Reference(None)) | None => {
                    return Err(Error::NullPointerException);
                }
                Some(_) => {
                    return Err(Error::TypeMismatch {
                        expected: "reference",
                        got: "other",
                    });
                }
            }
        }
        _ => {
            return Err(Error::Unimplemented {
                mnemonic: "unsupported lambda impl kind",
            });
        }
    };
    let impl_args = adapt_args_for_impl_desc(&impl_args, &lambda_info.impl_desc, heap);
    let result = execute_class(
        registry,
        loader,
        heap,
        output,
        &dispatch_class,
        &lambda_info.impl_method,
        &lambda_info.impl_desc,
        &impl_args,
    )?;
    let result = autobox_if_needed(
        result,
        &lambda_info.impl_desc,
        &lambda_info.sam_desc,
        heap,
    )?;
    Ok(Some(result))
}
