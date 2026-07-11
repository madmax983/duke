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
