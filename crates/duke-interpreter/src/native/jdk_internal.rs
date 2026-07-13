/// Native: `Unsafe.getUnsafe()Ljdk/internal/misc/Unsafe;` — returns a reference
/// to a (stateless) synthetic `Unsafe` instance. Every Duke `Unsafe` native
/// ignores `this`, so a freshly allocated zero-field object is sufficient; real
/// bytecode caches the returned reference in its own static field.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_unsafe_get_unsafe(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let unsafe_ref = heap.allocate("jdk/internal/misc/Unsafe".to_string(), 0);
    Ok(Some(Slot::Reference(Some(unsafe_ref))))
}
/// Native: `Unsafe.objectFieldOffset(Ljava/lang/Class;Ljava/lang/String;)J` —
/// returns the positional `fields` slot index of the named instance field.
pub(crate) fn native_unsafe_object_field_offset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 1)?;
    let field_name = extract_string_arg_value(args, 2, heap)?;
    let class_name = class_internal_name_from_ref(heap, class_ref)?;
    // The target class may only have been referenced via an `ldc` class constant
    // (mirror created) without its field layout being linked into the registry;
    // ensure it is loaded before resolving the positional field slot.
    ops.ensure_loaded(&class_name)?;
    let slot = ops.instance_field_slot(&class_name, &field_name)?;
    Ok(Some(Slot::Long(i64::try_from(slot).unwrap_or(0))))
}
/// Native: `Unsafe.arrayBaseOffset(Ljava/lang/Class;)I` — 0 under the positional
/// array model (element index == `fields` index).
pub(crate) fn native_unsafe_array_base_offset(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}
/// Native: `Unsafe.arrayIndexScale(Ljava/lang/Class;)I` — 1 under the positional
/// array model. It is a power of two, satisfying the real-bytecode invariant.
pub(crate) fn native_unsafe_array_index_scale(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(1)))
}
/// Native: `Unsafe.compareAndSetReference(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z`.
/// Reference-identity CAS on `fields[offset]`: if it equals `expected`, store `x`
/// and return true; otherwise return false.
pub(crate) fn native_unsafe_compare_and_set_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let x = args.get(4).copied().unwrap_or(Slot::Reference(None));
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?;
    if current.as_reference() == expected.as_reference() {
        heap.write_field(obj_ref, idx, x)?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.compareAndSetInt(Ljava/lang/Object;JII)Z`.
pub(crate) fn native_unsafe_compare_and_set_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = extract_int_arg(args, 3)?;
    let x = extract_int_arg(args, 4)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?.as_int().unwrap_or(0);
    if current == expected {
        heap.write_field(obj_ref, idx, Slot::Int(x))?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.compareAndSetLong(Ljava/lang/Object;JJJ)Z`.
pub(crate) fn native_unsafe_compare_and_set_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let expected = extract_long_arg(args, 3)?;
    let x = extract_long_arg(args, 4)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let current = unsafe_field_slot(heap, obj_ref, offset)?.as_long().unwrap_or(0);
    if current == expected {
        heap.write_field(obj_ref, idx, Slot::Long(x))?;
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Unsafe.getReferenceAcquire(Ljava/lang/Object;J)Ljava/lang/Object;`.
/// The single-threaded interpreter needs no memory ordering.
pub(crate) fn native_unsafe_get_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    Ok(Some(unsafe_field_slot(heap, obj_ref, offset)?))
}
/// Native: `Unsafe.putReferenceRelease(Ljava/lang/Object;JLjava/lang/Object;)V`.
pub(crate) fn native_unsafe_put_reference(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let x = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    heap.write_field(obj_ref, idx, x)?;
    Ok(None)
}
/// Native: `Unsafe.getAndAddInt(Ljava/lang/Object;JI)I` — atomically adds `delta`
/// to `fields[offset]` and returns the previous value.
pub(crate) fn native_unsafe_get_and_add_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let obj_ref = extract_ref_arg(args, 1)?;
    let offset = extract_long_arg(args, 2)?;
    let delta = extract_int_arg(args, 3)?;
    let idx = usize::try_from(offset).map_err(|_| Error::NullPointerException)?;
    let old = unsafe_field_slot(heap, obj_ref, offset)?.as_int().unwrap_or(0);
    heap.write_field(obj_ref, idx, Slot::Int(old.wrapping_add(delta)))?;
    Ok(Some(Slot::Int(old)))
}
/// Native: `sun/misc/Unsafe.allocateInstance(Ljava/lang/Class;)Ljava/lang/Object;`
/// — allocates a zero-initialised instance of the given class WITHOUT running any
/// constructor. gson uses this (reflectively, via `UnsafeAllocator`) to build POJOs
/// that lack a no-arg constructor. `args[0]` is the `Unsafe` receiver (ignored);
/// `args[1]` is the target `Class`.
pub(crate) fn native_unsafe_allocate_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let internal_name = class_internal_name_from_key(&class_key).to_string();
    ops.ensure_loaded(&internal_name)?;
    let instance_ref = ops.allocate_instance(heap, out, &class_key)?;
    Ok(Some(Slot::Reference(Some(instance_ref))))
}
/// Native: `jdk/internal/reflect/Reflection.getCallerClass()Ljava/lang/Class;`.
///
/// Real `HotSpot` semantics: return the `Class` of the method that called the
/// caller of `getCallerClass` — i.e. skip `getCallerClass`'s own (native) frame
/// and the immediate `@CallerSensitive` caller frame, returning the frame two
/// levels up (also skipping reflection/`MethodHandle` machinery frames).
///
/// Duke frame model: `control.stack_trace()` holds the Java frames captured at
/// the native call site, most-recent-first. `getCallerClass` is native and is
/// not itself represented in that snapshot, so:
///   `frames[0]` = the (`@CallerSensitive`) method that invoked getCallerClass
///                 (e.g. `ServiceLoader.load`)
///   `frames[1]` = that method's caller — the `Class` we must return.
/// We return the mirror for `frames[1]`. If the stack is too shallow (no
/// grand-caller, e.g. called directly from the entry frame) we fall back to
/// `frames[0]`, and to `null` only when the snapshot is empty. This is the
/// interpretation that lets `java/util/ServiceLoader.load(Ljava/lang/Class;)`
/// resolve its caller class under `DUKE_REAL_JDK=1`.
pub(crate) fn native_reflection_get_caller_class(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let frames = control.stack_trace();
    match frames.get(1).or_else(|| frames.first()) {
        Some(frame) => {
            let class_ref = allocate_class_object(heap, &frame.class_name)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        None => Ok(Some(Slot::Reference(None))),
    }
}
/// Native: `jdk/internal/misc/VM.initialize()V`.
///
/// In `HotSpot` this native saves the VM-supplied system properties into
/// `VM.savedProps` and records a few tuning constants; it does **not** itself
/// advance `VM.initLevel`. The boot sequence advances `initLevel` separately via
/// `VM.initLevel(int)` calls inside `System.initPhase1/2/3`, ending at
/// `SYSTEM_BOOTED` (== 4) by the time application code runs.
///
/// Duke never executes that boot sequence, yet application-time bytecode expects a
/// fully booted VM. In particular `java/util/ServiceLoader.<init>` calls
/// `VM.isBooted()`, which returns `initLevel >= SYSTEM_BOOTED`; when it reports
/// `false`, `ServiceLoader` takes its pre-boot module-bootstrap branch instead of
/// the normal construction path. `VM.<clinit>` is the one place that runs
/// `initialize()`, so we fold the boot progression into this native: it seeds
/// `initLevel = SYSTEM_BOOTED`, making `isBooted()` observe the booted state a
/// real application would see and letting `ServiceLoader` proceed normally. No
/// property map is materialized here — nothing on the reached path reads
/// `savedProps`; if a future path needs it, seed it at that point.
#[allow(clippy::unnecessary_wraps)] // signature must match `CallbackNativeHandler`
pub(crate) fn native_vm_initialize(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    // `SYSTEM_BOOTED` is the 4th init-level constant declared in
    // `jdk/internal/misc/VM` (JAVA_LANG_SYSTEM_INITED=1 .. SYSTEM_BOOTED=4).
    const SYSTEM_BOOTED: i32 = 4;
    ops.write_static_field("jdk/internal/misc/VM", "initLevel", Slot::Int(SYSTEM_BOOTED))?;
    Ok(None)
}
/// Native: `jdk/internal/reflect/Reflection.getClassAccessFlags(Ljava/lang/Class;)I`.
///
/// Returns the raw `ClassFile.access_flags` (§4.1) of the argument class. Real
/// `Reflection.verifyMemberAccess` — reached from `java.util.ServiceLoader` under
/// `real_jdk_shadow` after the module check — calls this and tests
/// `Modifier.isPublic(flags)` on the member's declaring class to decide access.
/// Returning the real flags (via [`CallbackOps::inspect_class`]) makes that check
/// observe the true `public`/non-public status; a synthetic stub with no classfile
/// flags falls back to `ACC_PUBLIC`.
pub(crate) fn native_reflection_get_class_access_flags(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let access_flags = ops
        .inspect_class(&internal_name)
        .map_or(SYNTHETIC_CLASS_ACCESS_FLAGS, |info| info.access_flags);
    Ok(Some(Slot::Int(i32::from(access_flags))))
}

// java.lang.invoke / SharedSecrets foundation
// ---------------------------------------------------------------------------
// Natives walking the real `FileOutputStream.<clinit>` chain under
// `DUKE_REAL_JDK=1`: `SharedSecrets.getJavaIOFileDescriptorAccess()` finds
// `FD_ACCESS == null`, which drives `MethodHandles.Lookup.ensureInitialized`
// and the `java.lang.invoke` bootstrap. These honor the real contracts (force
// real `<clinit>`, report real init state) rather than papering over the graph.

/// Native: `jdk/internal/misc/Unsafe.ensureClassInitialized(Ljava/lang/Class;)V`.
///
/// Real `MethodHandles.Lookup.ensureInitialized(Class)` — reached from
/// `FileOutputStream.<clinit>` via `SharedSecrets.getJavaIOFileDescriptorAccess`
/// → the `java.lang.invoke` bootstrap — calls `UNSAFE.ensureClassInitialized`
/// (JDK 21: `MethodHandles.Lookup.ensureInitialized` routes through
/// `Unsafe.ensureClassInitialized`) to force the target's `<clinit>` to complete
/// before a handle is bound against it. Duke honors that contract by driving the
/// argument class through [`CallbackOps::ensure_class_initialized`], which runs
/// `<clinit>` exactly once. `arg 0` is the (ignored) synthetic `Unsafe` receiver;
/// `arg 1` is the target `Class`.
pub(crate) fn native_unsafe_ensure_class_initialized(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 1)?;
    let class_name = class_internal_name_from_ref(heap, class_ref)?;
    ops.ensure_class_initialized(heap, out, &class_name)?;
    Ok(None)
}

/// Native: `initIDs()V` for `java/io/FileDescriptor` and `java/io/FileOutputStream`
/// (and the other file-stream classes) — no-op.
///
/// This is the opening instruction of each of those classes' real `<clinit>`
/// (reached once `Unsafe.ensureClassInitialized` forces initialization). In
/// `HotSpot` `initIDs` only caches the `jfieldID`s (e.g. `fd`/`handle`/`append`) so
/// later native code can poke those slots directly. Duke resolves fields
/// positionally by name, so there is nothing to cache — an honest no-op. The rest
/// of each `<clinit>` (installing the `JavaIOFileDescriptorAccess` via
/// `SharedSecrets`, building `in`/`out`/`err`, seeding `FD_ACCESS`) runs as real
/// bytecode.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_io_init_ids_noop(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(None)
}

/// Native: `java/io/FileDescriptor.getHandle(I)J`.
///
/// Called by the real `FileDescriptor(int)` constructor to seed the `handle`
/// field. Handles are a Windows-only concept; the unix JDK native returns `-1`
/// unconditionally (see `FileDescriptor_md.c`), so this mirrors that exactly.
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_file_descriptor_get_handle(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Long(-1)))
}

/// Native: `java/io/FileDescriptor.getAppend(I)Z`.
///
/// Called by the real `FileDescriptor(int)` constructor to seed the `append`
/// field. The unix JDK native returns `fcntl(fd, F_GETFL) & O_APPEND`. This path
/// runs only for the standard descriptors `0`/`1`/`2` built in
/// `FileDescriptor.<clinit>` — none of which is opened in append mode — so the
/// honest answer is `false`. (`FileOutputStream`'s own append flag lives in a
/// separate field set from its constructor, not from this descriptor query.)
#[allow(clippy::unnecessary_wraps)] // signature must match `NativeHandler`
pub(crate) fn native_file_descriptor_get_append(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    Ok(Some(Slot::Int(0)))
}

/// Native: `jdk/internal/access/JavaLangAccess.encodeASCII([CI[BII)I`.
///
/// The ASCII fast-path that `sun.nio.cs.UTF_8$Encoder.encodeArrayLoop` invokes via
/// `SharedSecrets.getJavaLangAccess()`. In the JDK this delegates to
/// `StringCoding.implEncodeAsciiArray`: copy chars from `sa[sp..]` into `da[dp..]`
/// as bytes, stopping at the first non-ASCII char (`>= 0x80`), and return the
/// number of chars encoded so the caller can advance its buffer positions and hand
/// the remainder to the multibyte slow path.
///
/// Args (interface dispatch on the placeholder `JavaLangAccess` object): `args[0]`
/// is the receiver; `args[1..=5]` are `sa`, `sp`, `da`, `dp`, `len`. Char-array
/// elements are stored as `Slot::Int` (widened, matching `caload`); byte-array
/// elements are written via `java_byte_slot`.
// Intentional narrowing: a `char` is 16-bit, so `i32 -> u16` models `caload`
// (dropping the sign/high bits Duke widens on push); the `u32 -> u8` write is exact
// because it only runs after the `< 0x80` ASCII guard.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn native_java_lang_access_encode_ascii(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let sa_ref = extract_ref_arg(args, 1)?;
    let sp = extract_int_arg(args, 2)?;
    let da_ref = extract_ref_arg(args, 3)?;
    let dp = extract_int_arg(args, 4)?;
    let len = extract_int_arg(args, 5)?;

    let mut count: i32 = 0;
    while count < len {
        let si = usize::try_from(sp + count).map_err(|_| index_out_of_bounds_error())?;
        let ch = {
            let fields = &heap.get(sa_ref)?.fields;
            let slot = fields.get(si).ok_or_else(index_out_of_bounds_error)?;
            u32::from(slot.as_int()? as u16)
        };
        if ch >= 0x80 {
            break;
        }
        let di = usize::try_from(dp + count).map_err(|_| index_out_of_bounds_error())?;
        heap.write_field(da_ref, di, java_byte_slot(ch as u8))?;
        count += 1;
    }
    Ok(Some(Slot::Int(count)))
}

/// `jdk/internal/access/SharedSecrets`, `.javaLangAccess` field name.
const SHARED_SECRETS: &str = "jdk/internal/access/SharedSecrets";
const JAVA_LANG_ACCESS_FIELD: &str = "javaLangAccess";

/// Ensure `SharedSecrets.javaLangAccess` is non-null before the file-write path
/// first reaches `jdk/internal/misc/Blocker`.
///
/// `Blocker.<clinit>` reads `SharedSecrets.getJavaLangAccess()` into its private
/// `JLA` field and asserts it is non-null — throwing `InternalError:
/// "JavaLangAccess not setup"` otherwise. The real JDK establishes that invariant
/// early in `System.initPhase1`, which calls `System.setJavaLangAccess(...)` with
/// the `java.lang.System$…` implementation. Duke keeps `java/lang/System` on the
/// `KEEP_SYNTHETIC` allowlist and never runs that boot sequence, so the field
/// stays null and every `FileOutputStream.write(...)` — which brackets its native
/// I/O in `Blocker.begin()`/`Blocker.end(...)` — would die in `Blocker.<clinit>`.
///
/// `Blocker` never invokes a `JavaLangAccess` method: `JLA` is read only as the
/// non-null boot-sanity check above (`begin`/`end` use `CarrierThread`, not
/// `JLA`). A minimal placeholder therefore satisfies the invariant honestly — and
/// if a future path does call a real `JavaLangAccess` method on it, that surfaces
/// as a legible method-not-found wall against `jdk/internal/access/JavaLangAccess`
/// rather than silent corruption. Idempotent: if the field is already installed
/// (e.g. a prior stream), this is a no-op.
fn ensure_java_lang_access_installed(
    heap: &mut duke_gc::Heap,
    ops: &mut dyn CallbackOps,
) -> Result<()> {
    ops.ensure_loaded(SHARED_SECRETS)?;
    if matches!(
        ops.read_static_field(SHARED_SECRETS, JAVA_LANG_ACCESS_FIELD)?,
        Slot::Reference(Some(_))
    ) {
        return Ok(());
    }
    let jla = heap.allocate("jdk/internal/access/JavaLangAccess".to_string(), 0);
    ops.write_static_field(
        SHARED_SECRETS,
        JAVA_LANG_ACCESS_FIELD,
        Slot::Reference(Some(jla)),
    )
}

/// Native: `java/io/FileOutputStream.initIDs()V`.
///
/// The opening instruction of the real `FileOutputStream.<clinit>` (forced once by
/// `Unsafe.ensureClassInitialized`, see [`native_unsafe_ensure_class_initialized`]).
/// Like every other `initIDs`, `HotSpot` uses it only to cache jfieldIDs, which Duke
/// resolves positionally — so there is nothing to cache. Duke additionally uses
/// this guaranteed pre-write seam to install the placeholder `JavaLangAccess`
/// ([`ensure_java_lang_access_installed`]): `FileOutputStream.<clinit>` already
/// loaded `SharedSecrets` at its `@0` `getJavaIOFileDescriptorAccess()` call, and
/// always completes before any instance's `write(...)` reaches `Blocker`.
pub(crate) fn native_file_output_stream_init_ids(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    ensure_java_lang_access_installed(heap, ops)?;
    Ok(None)
}

/// Native: `java/io/FileOutputStream.writeBytes([BIIZ)V`.
///
/// The leaf write native for the real-layout `FileOutputStream`. Real
/// `FileOutputStream.write(byte[])` computes the append flag, brackets the call in
/// `Blocker.begin()`/`end()`, and invokes this native with `(bytes, off, len,
/// append)`. Duke resolves the target file descriptor honestly from the object
/// graph: `this.fd` (a real `java/io/FileDescriptor`) → its `fd` int. The standard
/// descriptors built in `FileDescriptor.<clinit>` carry `fd == 1` (`out`) and
/// `fd == 2` (`err`), which Duke routes to the interpreter's stdout sink and the
/// host stderr respectively. Path-backed descriptors (opened via `open0`, not yet
/// wired) are not modelled here yet and surface as an explicit unsupported-fd wall.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
pub(crate) fn native_real_file_output_stream_write_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let array_ref = extract_ref_arg(args, 1)?;
    let offset = extract_int_arg(args, 2)?;
    let len = extract_int_arg(args, 3)?;

    let fd = file_output_stream_fd(heap, ops, this_ref)?;

    // Slice the byte array positionally (each element is a `Slot::Int` byte).
    if offset < 0 || len < 0 {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let (offset, len) = (offset as usize, len as usize);
    let fields = &heap.get(array_ref)?.fields;
    if offset > fields.len() || len > fields.len().saturating_sub(offset) {
        return Err(Error::JavaException {
            class_name: "java/lang/IndexOutOfBoundsException".to_string(),
        });
    }
    let bytes: Vec<u8> = fields[offset..offset + len]
        .iter()
        .map(|slot| match slot {
            Slot::Int(v) => *v as u8,
            _ => 0,
        })
        .collect();

    match fd {
        1 => {
            out.write_all(&bytes).ok();
        }
        2 => {
            use std::io::Write as _;
            std::io::stderr().write_all(&bytes).ok();
        }
        _ => {
            // Path-backed descriptors (opened via `open0`) are not modelled yet;
            // only the standard stdout/stderr descriptors are wired.
            return Err(Error::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    }
    Ok(None)
}

/// Read `this.fd.fd` (the OS descriptor int) from a real-layout `FileOutputStream`.
fn file_output_stream_fd(
    heap: &duke_gc::Heap,
    ops: &mut dyn CallbackOps,
    this_ref: u64,
) -> Result<i32> {
    let fd_desc = ops.read_instance_field(heap, this_ref, "java/io/FileOutputStream", "fd")?;
    let Slot::Reference(Some(fd_ref)) = fd_desc else {
        return Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        });
    };
    match ops.read_instance_field(heap, fd_ref, "java/io/FileDescriptor", "fd")? {
        Slot::Int(v) => Ok(v),
        _ => Err(Error::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
    }
}
