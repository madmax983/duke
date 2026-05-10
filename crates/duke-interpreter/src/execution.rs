//! The Core JVM Execution Engine.
//!
//! This module contains the main interpretation loop for the JVM.
//! It is responsible for taking a decoded stream of bytecodes and
//! executing them sequentially against the current thread's frame stack
//! and the global garbage-collected heap.
//!
//! The engine implements a stack-based machine model, handling everything
//! from basic arithmetic to complex method dispatch, object allocation,
//! and garbage collection safe points.

use std::io::Write;

use duke_bytecode::Instruction;
use duke_classfile::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Error, Frame, Result, Slot};

use crate::CachedDispatch;
use crate::registry::ClassRegistry;
#[allow(clippy::wildcard_imports)]
use crate::*;

/// Executes the active method's bytecode instructions to completion or exception.
///
/// This is the central run-loop of the interpreter. It continually fetches the
/// next instruction pointed to by the active frame's Program Counter (PC),
/// executes its operational semantics (modifying the local variables, operand stack,
/// or global heap), and advances the PC.
///
/// If a method invokes another Java method, a new [`Frame`] is pushed onto the
/// call stack and execution jumps to the callee. Native methods are dispatched
/// to their Rust implementations directly.
///
/// # Examples
///
/// ```ignore
/// use duke_interpreter::{ExecutionState, ClassRegistry, run_execution};
/// use duke_gc::Heap;
/// use duke_loader::BootstrapLoader;
///
/// let mut registry = ClassRegistry::new();
/// let loader = BootstrapLoader::new(vec![]);
/// let mut heap = Heap::new();
/// let mut state = ExecutionState::new();
/// let mut stdout = std::io::stdout();
///
/// // Start the execution loop (assuming state has an active frame)
/// let result = run_execution(&mut state, &mut registry, &loader, &mut heap, &mut stdout);
/// ```
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::cognitive_complexity,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
pub fn run_execution(
    state: &mut ExecutionState,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    gc_allowed: bool,
    quantum: Option<usize>,
) -> Result<ExecutionOutcome> {
    let ExecutionState {
        current_class,
        method_idx,
        pc_to_idx,
        instructions,
        frame,
        call_stack,
        frame_pool,
        dispatch_cache,
        vtable_cache,
        idx,
        string_intern,
        #[cfg(feature = "telemetry")]
        current_method,
    } = state;

    let mut remaining = quantum.unwrap_or(usize::MAX);

    loop {
        if remaining == 0 {
            return Ok(ExecutionOutcome::Yield);
        }
        remaining = remaining.saturating_sub(1);

        let (pc, instr) = {
            let Some(&(pc, ref instr)) = instructions.get(*idx) else {
                return Err(Error::FellOffEnd);
            };
            (pc, instr.clone())
        };

        if std::env::var_os("DUKE_TRACE_EXEC").is_some() {
            let method_name = registry
                .get(current_class)
                .ok()
                .and_then(|ctx| ctx.methods.get(*method_idx))
                .map_or("<unknown>", |method| method.name.as_str());
            eprintln!(
                "duke: trace {}::{}@{} {} stack={}",
                current_class,
                method_name,
                pc,
                instr.mnemonic(),
                frame.stack_len()
            );
        }

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                *idx = *pc_to_idx
                    .get(&target)
                    .ok_or(Error::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        // Helper macro for return instructions: pop call stack or return to Rust.
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok(ExecutionOutcome::Returned($val)),
                    Some(caller) => {
                        let ret_val = $val;
                        // Recycle callee frame buffers before overwriting `frame`.
                        let old = std::mem::replace(frame, caller.frame);
                        let (l, s) = old.into_pool_bufs();
                        frame_pool.release(l, s);
                        *method_idx = caller.method_idx;
                        *pc_to_idx = caller.pc_to_idx;
                        *idx = caller.resume_idx;
                        *current_class = caller.class_name;
                        *instructions = std::sync::Arc::clone(
                            &registry.get(&current_class)?.methods[*method_idx].instructions,
                        );
                        #[cfg(feature = "telemetry")]
                        {
                            *current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(*method_idx)
                                        .map(|m| m.name.clone())
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

        macro_rules! propagate_java_exception {
            ($exc_class_name:expr, $exception_ref:expr, $throw_pc:expr) => {{
                let exc_class_name = $exc_class_name;
                let exception_ref = $exception_ref;

                #[cfg(feature = "telemetry")]
                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
                    &exc_class_name,
                    &current_class,
                    &current_method,
                    $throw_pc,
                );

                // Clone the exception table to release the borrow on registry,
                // so find_exception_handler can use &mut registry for hierarchy checks.
                // ⚡ Bolt: `ExceptionEntry` now derives `Clone`, avoiding manual field-by-field mapping.
                let exc_table = registry.get(&current_class)?.methods[*method_idx]
                    .exception_table
                    .clone();
                let handler = find_exception_handler(
                    &exc_table,
                    $throw_pc,
                    &exc_class_name,
                    current_class,
                    registry,
                    loader,
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
                    *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        Error::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                loop {
                    match call_stack.pop() {
                        None => {
                            record_uncaught_java_exception_ref(&exc_class_name, exception_ref);
                            return Err(Error::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            // Recycle callee frame buffers before overwriting `frame` —
                            // mirrors the do_return! pattern to avoid a pool leak.
                            let old = std::mem::replace(frame, caller.frame);
                            let (l, s) = old.into_pool_bufs();
                            frame_pool.release(l, s);
                            *method_idx = caller.method_idx;
                            *pc_to_idx = caller.pc_to_idx;
                            *current_class = caller.class_name;
                            *instructions = std::sync::Arc::clone(
                                &registry.get(&current_class)?.methods[*method_idx].instructions,
                            );
                            #[cfg(feature = "telemetry")]
                            {
                                *current_method = registry
                                    .get(&current_class)
                                    .map(|c| {
                                        c.methods
                                            .get(*method_idx)
                                            .map(|m| m.name.clone())
                                            .unwrap_or_default()
                                    })
                                    .unwrap_or_default();
                            }

                            let (caller_exc_table, caller_pc) = {
                                let ctx = registry.get(&current_class)?;
                                let cpc = if caller.resume_idx > 0 {
                                    ctx.methods[*method_idx].instructions[caller.resume_idx - 1].0
                                } else {
                                    0
                                };
                                // ⚡ Bolt: `ExceptionEntry` now derives `Clone`, avoiding manual field-by-field mapping.
                                let tbl = ctx.methods[*method_idx]
                                    .exception_table
                                    .clone();
                                (tbl, cpc)
                            };
                            let handler = find_exception_handler(
                                &caller_exc_table,
                                caller_pc,
                                &exc_class_name,
                                current_class,
                                registry,
                                loader,
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
                                *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    Error::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                        }
                    }
                }
                continue;
            }};
        }
        // Helper macro for native call results
        macro_rules! handle_native_result {
            ($res:expr, $reg:expr, $ld:expr, $hp:expr, $p:expr) => {
                match $res {
                    Ok(r) => r,
                    Err(Error::JavaException { class_name }) => {
                        let exception_ref =
                            materialize_java_exception_object($reg, $ld, $hp, &class_name)?;
                        propagate_java_exception!(class_name, exception_ref, $p);
                    }
                    Err(err) => {
                        let class_name = match err {
                            Error::NullPointerException => {
                                "java/lang/NullPointerException".to_string()
                            }
                            Error::ClassCastException { .. } => {
                                "java/lang/ClassCastException".to_string()
                            }
                            Error::ArrayIndexOutOfBounds { .. } => {
                                "java/lang/ArrayIndexOutOfBoundsException".to_string()
                            }
                            Error::JavaException { class_name } => class_name,
                            other => return Err(other),
                        };
                        let exception_ref =
                            materialize_java_exception_object($reg, $ld, $hp, &class_name)?;
                        propagate_java_exception!(class_name, exception_ref, $p);
                    }
                }
            };
        }

        macro_rules! throw_java {
            ($class_name:expr) => {{
                let class_name = $class_name.to_string();
                let exception_ref =
                    materialize_java_exception_object(registry, loader, heap, &class_name)?;
                propagate_java_exception!(class_name, exception_ref, pc);
            }};
        }

        // Telemetry: capture opcode name and start time before dispatch.
        // Arms that use `continue` (branches, invokes) will skip the post-match
        // recording for that iteration — timing is approximate for those opcodes.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let (_telem_name, _telem_pc, _telem_start) = {
            let name = instr_name(&instr);
            let pc_val = pc;
            (name, pc_val, std::time::Instant::now())
        };

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                if let Some(cached) = dispatch_cache
                    .get(current_class.as_str())
                    .and_then(|m| m.get(&cp_idx.0))
                {
                    // Fast path: cache hit — zero registry lookups.
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(cached.max_locals, Slot::Int(0));
                    if cached.arg_count > cached.max_locals {
                        return Err(Error::LocalOutOfBounds {
                            index: cached.arg_count,
                            max_locals: cached.max_locals,
                        });
                    }
                    pop_typed_args_into_locals(&cached.param_types, frame, &mut locals_buf, 0)?;
                    let callee_frame =
                        Frame::from_pool_bufs(locals_buf, stack_buf, cached.max_stack);
                    match activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        cached.class_name.clone(),
                        cached.method_idx,
                        std::sync::Arc::clone(&cached.pc_to_idx),
                        callee_frame,
                        std::sync::Arc::clone(&cached.instructions),
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        registry,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    ) {
                        Ok(()) => {}
                        Err(Error::JavaException { class_name }) => {
                            throw_java!(class_name);
                        }
                        Err(other) => return Err(other),
                    }
                    *idx = 0;
                    continue;
                }
                // Slow path: full CP resolution + method search.
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let class_was_loaded = registry.ensure_loaded_from(
                    &callee_class,
                    Some(current_class.as_str()),
                    loader,
                )?;
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                if class_was_loaded {
                    ensure_initialized(
                        registry,
                        loader,
                        heap,
                        stdout,
                        &callee_class_key,
                        current_class,
                    )?;
                }
                let callee_idx = if has_registered_native_override(
                    registry,
                    &callee_class_key,
                    &callee_name,
                    &callee_desc,
                ) {
                    None
                } else {
                    let ctx = registry.get(&callee_class_key)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                            let ctx = registry.get(&callee_class_key)?;
                            let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                            let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                            let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                            let instrs =
                                std::sync::Arc::clone(&ctx.methods[callee_idx].instructions);
                            // Populate cache with all pre-resolved data.
                            dispatch_cache
                                .entry(current_class.clone())
                                .or_default()
                                .insert(
                                    cp_idx.0,
                                    CachedDispatch {
                                        class_name: callee_class_key.clone(),
                                        method_idx: callee_idx,
                                        arg_count,
                                        param_types: parse_arg_types(&callee_desc),
                                        max_locals,
                                        max_stack,
                                        pc_to_idx: std::sync::Arc::clone(&pci),
                                        instructions: std::sync::Arc::clone(&instrs),
                                    },
                                );
                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                            locals_buf.resize(max_locals, Slot::Int(0));
                            if arg_count > max_locals {
                                return Err(Error::LocalOutOfBounds {
                                    index: arg_count,
                                    max_locals,
                                });
                            }
                            pop_typed_args_into_locals(
                                &parse_arg_types(&callee_desc),
                                frame,
                                &mut locals_buf,
                                0,
                            )?;
                            let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                            (pci, instrs, f)
                        };
                        match activate_method_state(
                            frame,
                            method_idx,
                            pc_to_idx,
                            instructions,
                            current_class,
                            call_stack,
                            callee_class_key,
                            callee_idx,
                            callee_pc_to_idx,
                            callee_frame,
                            callee_instructions,
                            *idx + 1,
                            #[cfg(feature = "telemetry")]
                            registry,
                            #[cfg(feature = "telemetry")]
                            current_method,
                        ) {
                            Ok(()) => {}
                            Err(Error::JavaException { class_name }) => {
                                throw_java!(class_name);
                            }
                            Err(other) => return Err(other),
                        }
                        *idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        let handler_kind = lookup_registered_native_kind(
                            registry,
                            &callee_class_key,
                            &callee_name,
                            &callee_desc,
                        );
                        match handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    registry.internal_name_for_class(&callee_class_key),
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &native_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    registry.internal_name_for_class(&callee_class_key),
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &native_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                return Err(Error::MethodNotFound {
                                    name: format!(
                                        "{}.{callee_name}",
                                        registry.internal_name_for_class(&callee_class_key)
                                    ),
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
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(Error::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = heap.allocate_string(intern_key.1.clone());
                        string_intern.insert(intern_key, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        let intern_key = (1, class_name);
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = allocate_class_object(heap, &intern_key.1)?;
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, cp_idx)?;
                    }
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                let string_info = {
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(Error::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let intern_key = (0, s);
                    let r = if let Some(&cached) = string_intern.get(&intern_key) {
                        cached
                    } else {
                        let r = heap.allocate_string(intern_key.1.clone());
                        string_intern.insert(intern_key, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        let intern_key = (1, class_name);
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = allocate_class_object(heap, &intern_key.1)?;
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, idx_val)?;
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
                    throw_java!("java/lang/ArithmeticException");
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    throw_java!("java/lang/ArithmeticException");
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
                    throw_java!("java/lang/ArithmeticException");
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    throw_java!("java/lang/ArithmeticException");
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
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                // Walk the super chain to sum all instance field counts
                // (e.g. Enum has 2 fields inherited by every enum subclass).
                let field_count = total_instance_field_count(registry, &target_class_key);
                #[cfg(feature = "telemetry")]
                registry.telemetry.object_lineage.record(
                    current_class,
                    current_method,
                    pc,
                    &target_class_key,
                );
                let r = heap.allocate(target_class_key.clone(), field_count);
                // Set reference/long/float/double fields to their JVM-spec defaults.
                // heap.allocate initialises everything to Int(0), which is wrong
                // for reference-typed fields (should be Reference(None)).
                init_object_fields(registry, heap, r, &target_class_key);
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let r = frame.pop_ref()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                let fidx = field_slot_idx(registry, &target_class_key, &field_name)?;
                let val = heap.get(r)?.fields[fidx];
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                let fidx = field_slot_idx(registry, &target_class_key, &field_name)?;
                heap.write_field(r, fidx, val)?;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class_key)?, &field_name)?;
                let val = registry.get(&target_class_key)?.static_fields[sidx];
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let target_class_key =
                    registry.class_key_from_source(&target_class, Some(current_class.as_str()));
                let val = frame.pop()?;
                registry.ensure_loaded_from(&target_class, Some(current_class.as_str()), loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class_key,
                    current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class_key)?, &field_name)?;
                registry.get_mut(&target_class_key)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                // Fast path: cache hit for invokespecial (static dispatch — safe to cache).
                if matches!(instr, Instruction::Invokespecial(_))
                    && let Some(cached) = dispatch_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                {
                    // Zero registry lookups — all data pre-cached.
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(cached.max_locals, Slot::Int(0));
                    if cached.arg_count + 1 > cached.max_locals {
                        return Err(Error::LocalOutOfBounds {
                            index: cached.arg_count + 1,
                            max_locals: cached.max_locals,
                        });
                    }
                    pop_typed_args_into_locals(&cached.param_types, frame, &mut locals_buf, 1)?;
                    locals_buf[0] = frame.pop()?; // `this`
                    let callee_frame =
                        Frame::from_pool_bufs(locals_buf, stack_buf, cached.max_stack);
                    match activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        cached.class_name.clone(),
                        cached.method_idx,
                        std::sync::Arc::clone(&cached.pc_to_idx),
                        callee_frame,
                        std::sync::Arc::clone(&cached.instructions),
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        registry,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    ) {
                        Ok(()) => {}
                        Err(Error::JavaException { class_name }) => {
                            throw_java!(class_name);
                        }
                        Err(other) => return Err(other),
                    }
                    *idx = 0;
                    continue;
                }
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                // <clinit> (static initialiser) is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    *idx += 1;
                    continue;
                }
                // Attempt to load the target class; soft-fail for unloadable.
                let class_was_loaded = registry.ensure_loaded_from(
                    &callee_class,
                    Some(current_class.as_str()),
                    loader,
                )?;
                let virtual_start: Option<String> =
                    if matches!(instr, Instruction::Invokevirtual(_)) {
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        if stack_len > arg_count {
                            let this_pos = stack_len - arg_count - 1;
                            if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                                heap.get(r).ok().map(|o| o.class_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                if matches!(instr, Instruction::Invokevirtual(_))
                    && let Some(runtime_class) = virtual_start.as_ref()
                    && annotation_proxy_type(runtime_class).is_some()
                {
                    let arg_count = parse_arg_count(&callee_desc);
                    let stack_len = frame.stack_len();
                    let this_pos = stack_len - arg_count - 1;
                    let Slot::Reference(Some(receiver_ref)) = frame.peek_at(this_pos)? else {
                        return Err(Error::NullPointerException);
                    };
                    if let Some(result) = annotation_proxy_element_slot(
                        heap,
                        receiver_ref,
                        &callee_name,
                        &callee_desc,
                    )? {
                        for _ in 0..arg_count {
                            frame.pop()?;
                        }
                        frame.pop()?;
                        if method_return_descriptor(&callee_desc) != "V" {
                            frame.push(result)?;
                        }
                        *idx += 1;
                        continue;
                    }
                }
                // vtable fast path for invokevirtual — check PIC after receiver type is known.
                if matches!(instr, Instruction::Invokevirtual(_))
                    && let Some(ref runtime_class) = virtual_start
                    && let Some(cached) = vtable_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                        .and_then(|m| m.get(runtime_class.as_str()))
                {
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(cached.max_locals, Slot::Int(0));
                    if cached.arg_count + 1 > cached.max_locals {
                        return Err(Error::LocalOutOfBounds {
                            index: cached.arg_count + 1,
                            max_locals: cached.max_locals,
                        });
                    }
                    pop_typed_args_into_locals(&cached.param_types, frame, &mut locals_buf, 1)?;
                    locals_buf[0] = frame.pop()?; // `this`
                    let callee_frame =
                        Frame::from_pool_bufs(locals_buf, stack_buf, cached.max_stack);
                    match activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        cached.class_name.clone(),
                        cached.method_idx,
                        std::sync::Arc::clone(&cached.pc_to_idx),
                        callee_frame,
                        std::sync::Arc::clone(&cached.instructions),
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        registry,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    ) {
                        Ok(()) => {}
                        Err(Error::JavaException { class_name }) => {
                            throw_java!(class_name);
                        }
                        Err(other) => return Err(other),
                    }
                    *idx = 0;
                    continue;
                }
                let resolved = if let Some(runtime_class) = virtual_start.as_ref() {
                    match resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        runtime_class,
                        &callee_name,
                        &callee_desc,
                    ) {
                        MethodHierarchyLookup::Bytecode(class_name, method_idx) => {
                            MethodHierarchyLookup::Bytecode(class_name, method_idx)
                        }
                        MethodHierarchyLookup::NativeOverride => {
                            MethodHierarchyLookup::NativeOverride
                        }
                        MethodHierarchyLookup::Missing if class_was_loaded => {
                            resolve_method_in_hierarchy_lookup(
                                registry,
                                loader,
                                &callee_class_key,
                                &callee_name,
                                &callee_desc,
                            )
                        }
                        MethodHierarchyLookup::Missing => MethodHierarchyLookup::Missing,
                    }
                } else if class_was_loaded {
                    resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        &callee_class_key,
                        &callee_name,
                        &callee_desc,
                    )
                } else {
                    MethodHierarchyLookup::Missing
                };
                let allow_lambda_dispatch = matches!(resolved, MethodHierarchyLookup::Missing);
                let (dispatch_class, callee_idx) = match resolved {
                    MethodHierarchyLookup::Bytecode(cls, i) => (cls, i),
                    MethodHierarchyLookup::NativeOverride | MethodHierarchyLookup::Missing => {
                        // Check lambda dispatch before native fallback.
                        if allow_lambda_dispatch {
                            let arg_count = parse_arg_count(&callee_desc);
                            let stack_len = frame.stack_len();
                            if stack_len > arg_count
                                && let Some(ref actual_class) = virtual_start
                                && let Some(lambda_info) =
                                    registry.get_lambda(actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut sam_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    sam_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                let this_ref = match &this_slot {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(Error::NullPointerException),
                                };

                                let obj = heap.get(this_ref)?;
                                // ⚡ Bolt: Pre-allocate vector capacity to avoid multiple reallocations during push/extend
                                let mut impl_args: Vec<Slot> =
                                    Vec::with_capacity(lambda_info.captured_count + sam_args.len());
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(sam_args.iter().copied());

                                let _ = registry.ensure_loaded_from(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                    loader,
                                );
                                let impl_class_key = registry.class_key_from_source(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                );

                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let instrs = std::sync::Arc::clone(
                                            &ctx.methods[impl_idx].instructions,
                                        );
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, instrs, f)
                                    };
                                    match activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        callee_instructions,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        registry,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    ) {
                                        Ok(()) => {}
                                        Err(Error::JavaException { class_name }) => {
                                            throw_java!(class_name);
                                        }
                                        Err(other) => return Err(other),
                                    }
                                    *idx = 0;
                                    continue;
                                }
                            }
                        }
                        // Check native registry, walking the super chain.
                        let native_handler_kind = {
                            let mut found = None;

                            // Walk from runtime class first (invokevirtual virtual dispatch).
                            if let Some(ref runtime_class) = virtual_start {
                                let mut sc: Option<String> = Some(runtime_class.clone());
                                while let Some(ref s) = sc {
                                    if let Some(h) = lookup_registered_native_kind(
                                        registry,
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }

                            // Fall back to declared (callee_class) super chain.
                            if found.is_none() {
                                found = lookup_registered_native_kind(
                                    registry,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                            }
                            if found.is_none() {
                                // Walk super chain for native lookup (e.g. Enum.ordinal
                                // called via SimpleEnum$Color.ordinal).
                                let start = if callee_class_key.starts_with('[') {
                                    // Arrays inherit from Object.
                                    Some("java/lang/Object".to_string())
                                } else {
                                    registry
                                        .get(&callee_class_key)
                                        .ok()
                                        .and_then(|c| c.super_class.clone())
                                };
                                let mut sc = start;
                                while let Some(ref s) = sc {
                                    if let Some(h) = lookup_registered_native_kind(
                                        registry,
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }
                            found
                        };
                        match native_handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        // Native methods do not perform a bytecode hierarchy
                                        // walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            registry.internal_name_for_class(&callee_class_key),
                                            false,
                                        );
                                    }
                                }
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &native_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            registry.internal_name_for_class(&callee_class_key),
                                            false,
                                        );
                                    }
                                }
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &native_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                if !class_was_loaded {
                                    // Unloadable target — preserve legacy soft-fail behavior.
                                    let arg_count = parse_arg_count(&callee_desc);
                                    for _ in 0..arg_count {
                                        frame.pop()?;
                                    }
                                    frame.pop()?; // pop `this`
                                    *idx += 1;
                                    continue;
                                }
                                return Err(Error::MethodNotFound {
                                    name: format!(
                                        "{}.{callee_name}",
                                        registry.internal_name_for_class(
                                            virtual_start
                                                .as_deref()
                                                .unwrap_or(callee_class_key.as_str()),
                                        )
                                    ),
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                #[cfg(feature = "telemetry")]
                if matches!(instr, Instruction::Invokevirtual(_)) {
                    registry.telemetry.dispatch_resolution.record(
                        current_class,
                        cp_idx.0,
                        &dispatch_class,
                        dispatch_class != callee_class_key,
                    );
                }
                let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let instrs = std::sync::Arc::clone(&ctx.methods[callee_idx].instructions);
                    // Populate dispatch cache for invokespecial (static dispatch — stable result).
                    // Also populate vtable cache for invokevirtual when receiver type is known.
                    if matches!(instr, Instruction::Invokespecial(_)) {
                        dispatch_cache
                            .entry(current_class.clone())
                            .or_default()
                            .insert(
                                cp_idx.0,
                                CachedDispatch {
                                    class_name: dispatch_class.clone(),
                                    method_idx: callee_idx,
                                    arg_count,
                                    param_types: parse_arg_types(&callee_desc),
                                    max_locals,
                                    max_stack,
                                    pc_to_idx: std::sync::Arc::clone(&pci),
                                    instructions: std::sync::Arc::clone(&instrs),
                                },
                            );
                    } else if let Some(ref runtime_class) = virtual_start {
                        vtable_cache
                            .entry(current_class.clone())
                            .or_default()
                            .entry(cp_idx.0)
                            .or_default()
                            .insert(
                                runtime_class.clone(),
                                CachedDispatch {
                                    class_name: dispatch_class.clone(),
                                    method_idx: callee_idx,
                                    arg_count,
                                    param_types: parse_arg_types(&callee_desc),
                                    max_locals,
                                    max_stack,
                                    pc_to_idx: std::sync::Arc::clone(&pci),
                                    instructions: std::sync::Arc::clone(&instrs),
                                },
                            );
                    }
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(Error::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop args with wide-type-aware indexing (doubles/longs occupy 2 local slots).
                    pop_typed_args_into_locals(
                        &parse_arg_types(&callee_desc),
                        frame,
                        &mut locals_buf,
                        1,
                    )?;
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, instrs, f)
                };
                match activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    callee_instructions,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    registry,
                    #[cfg(feature = "telemetry")]
                    current_method,
                ) {
                    Ok(()) => {}
                    Err(Error::JavaException { class_name }) => {
                        throw_java!(class_name);
                    }
                    Err(other) => return Err(other),
                }
                *idx = 0;
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
                    return Err(Error::NegativeArraySize { size: count });
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
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(Error::NegativeArraySize { size: count });
                }
                let r = heap.allocate(array_type, count as usize);
                // Fix elements to Reference(None).
                let obj = heap.get_mut(r)?;
                for slot in &mut obj.fields {
                    *slot = Slot::Reference(None);
                }
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i8)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
                    }
                    i32::from(fields[idx_val as usize].as_int()? as u16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        throw_java!("java/lang/ArrayIndexOutOfBoundsException");
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
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        if is_assignable_from(
                            registry,
                            loader,
                            &actual,
                            &target,
                            Some(current_class.as_str()),
                        ) {
                            frame.push(slot)?;
                        } else {
                            throw_java!("java/lang/ClassCastException");
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
                        let target = {
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        let result = i32::from(is_assignable_from(
                            registry,
                            loader,
                            &actual,
                            &target,
                            Some(current_class.as_str()),
                        ));
                        frame.push(Slot::Int(result))?;
                    }
                    _ => {
                        return Err(Error::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow with exception table dispatch ----
            Instruction::Athrow => {
                let exception_ref = frame.pop_ref()?;
                let exc_class_name = heap.get(exception_ref)?.class_name.clone();
                propagate_java_exception!(exc_class_name, exception_ref, pc);
            }

            // ----------------------------------------------------------------
            // invokedynamic — resolve bootstrap method, dispatch based on
            // the bootstrap class (StringConcatFactory, LambdaMetafactory).
            // ----------------------------------------------------------------
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(current_class)?;
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
                        _ => return Err(Error::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_class, bsm_args) = {
                    let ctx = registry.get(current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(Error::InvalidCpIndex { index: bsm_idx })?;
                    let (_kind, class, _name, _desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<duke_classfile::CpIndex> = bsm_entry.arguments.clone();
                    (class, args)
                };

                let _ = &call_name; // suppress unused warning for now

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    let arg_count = parse_arg_count(&call_desc);
                    let arg_types = parse_arg_types(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut dynamic_args = vec![Slot::Int(0); arg_count];

                    for i in (0..arg_count).rev() {
                        dynamic_args[i] = frame.pop()?;
                    }

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if bsm_args.is_empty() {
                            String::new()
                        } else {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
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
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(Error::Unimplemented {
                                mnemonic: "LambdaMetafactory requires 3 bootstrap args",
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    let sam_method = call_name.clone();

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => {
                                        return Err(Error::InvalidCpIndex {
                                            index: descriptor_index.0 as usize,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(Error::InvalidCpIndex {
                                    index: bsm_args[0].0 as usize,
                                });
                            }
                        }
                    };

                    // Pop captured variables from the stack.
                    let captured_count = parse_arg_count(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut captured_args = vec![Slot::Int(0); captured_count];

                    for i in (0..captured_count).rev() {
                        captured_args[i] = frame.pop()?;
                    }

                    // Extract SAM interface from invokedynamic return type: "(...)Ljava/util/function/Function;" → "java/util/function/Function"
                    let sam_interface = call_desc
                        .split_once(')')
                        .and_then(|(_, ret)| ret.strip_prefix('L'))
                        .and_then(|s| s.strip_suffix(';'))
                        .unwrap_or("")
                        .to_string();

                    let lambda_info = LambdaInfo {
                        impl_class: impl_class.clone(),
                        impl_method: impl_method.clone(),
                        impl_desc: impl_desc.clone(),
                        impl_kind,
                        sam_method,
                        sam_desc,
                        sam_interface,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    let r = heap.allocate(lambda_class, captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    let _ = registry.ensure_loaded_from(
                        &impl_class,
                        Some(current_class.as_str()),
                        loader,
                    );

                    frame.push(Slot::Reference(Some(r)))?;
                    if gc_allowed && heap.should_gc() {
                        let roots = gather_roots(frame, call_stack, registry);
                        heap.collect(&roots);
                        patch_forwarded_slots(frame, call_stack, registry, heap);
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
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let callee_class_key =
                    registry.class_key_from_source(&callee_class, Some(current_class.as_str()));
                if callee_name == "<clinit>" {
                    *idx += 1;
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
                        heap.get(r).ok().map(|o| o.class_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| callee_class.clone());

                if annotation_proxy_type(&actual_class).is_some() {
                    let stack_len = frame.stack_len();
                    let this_pos = stack_len - arg_count - 1;
                    let Slot::Reference(Some(receiver_ref)) = frame.peek_at(this_pos)? else {
                        return Err(Error::NullPointerException);
                    };
                    if let Some(result) = annotation_proxy_element_slot(
                        heap,
                        receiver_ref,
                        &callee_name,
                        &callee_desc,
                    )? {
                        for _ in 0..arg_count {
                            frame.pop()?;
                        }
                        frame.pop()?;
                        if method_return_descriptor(&callee_desc) != "V" {
                            frame.push(result)?;
                        }
                        *idx += 1;
                        continue;
                    }
                }

                // Try to find the method on the actual class (walking hierarchy).
                let resolved = match resolve_method_in_hierarchy_lookup(
                    registry,
                    loader,
                    &actual_class,
                    &callee_name,
                    &callee_desc,
                ) {
                    MethodHierarchyLookup::Bytecode(class_name, method_idx) => {
                        MethodHierarchyLookup::Bytecode(class_name, method_idx)
                    }
                    MethodHierarchyLookup::NativeOverride => MethodHierarchyLookup::NativeOverride,
                    MethodHierarchyLookup::Missing => resolve_method_in_hierarchy_lookup(
                        registry,
                        loader,
                        &callee_class_key,
                        &callee_name,
                        &callee_desc,
                    ),
                };
                let allow_lambda_dispatch = matches!(resolved, MethodHierarchyLookup::Missing);

                let (dispatch_class, callee_idx) =
                    if let MethodHierarchyLookup::Bytecode(dispatch_class, callee_idx) = resolved {
                        (dispatch_class, callee_idx)
                    } else {
                        // Check native registry — try actual class then interface class.
                        let native_kind = lookup_registered_native_kind(
                            registry,
                            &actual_class,
                            &callee_name,
                            &callee_desc,
                        )
                        .or_else(|| {
                            lookup_registered_native_kind(
                                registry,
                                &callee_class_key,
                                &callee_name,
                                &callee_desc,
                            )
                        });
                        match native_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                // Native path: collect args + this into a Vec<Slot>
                                // for the handler(&[Slot], ...) signature.
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut callee_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }

                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result =
                                    handler(&callee_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    // Native interface methods skip the bytecode
                                    // hierarchy walk — hierarchy_walk is always false here.
                                    registry.telemetry.dispatch_resolution.record(
                                        current_class,
                                        cp_idx.0,
                                        &actual_class,
                                        false,
                                    );
                                }
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &callee_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut callee_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = native_control_for_call(
                                    registry,
                                    current_class,
                                    *method_idx,
                                    pc,
                                    call_stack,
                                    &callee_class_key,
                                    &callee_name,
                                    &callee_desc,
                                );
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &callee_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        registry.internal_name_for_class(&callee_class_key),
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    registry.telemetry.dispatch_resolution.record(
                                        current_class,
                                        cp_idx.0,
                                        &actual_class,
                                        false,
                                    );
                                }
                                let result =
                                    handle_native_result!(result, registry, loader, heap, pc);
                                if let Some(outcome) = finish_native_call(
                                    &mut native_control,
                                    frame,
                                    idx,
                                    result,
                                    &callee_args,
                                )? {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {} // fall through to lambda / no-op
                        }
                        // Check lambda registry for SAM dispatch.
                        if allow_lambda_dispatch
                            && let Some(lambda_info) = registry.get_lambda(&actual_class).cloned()
                            && callee_name == lambda_info.sam_method
                        {
                            // Lambda path: collect args + this into a Vec<Slot>.
                            // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                            let mut callee_args = vec![Slot::Int(0); arg_count];

                            for i in (0..arg_count).rev() {
                                callee_args[i] = frame.pop()?;
                            }
                            let this_slot = frame.pop()?;
                            callee_args.insert(0, this_slot);
                            let this_ref = match &callee_args[0] {
                                Slot::Reference(Some(r)) => *r,
                                _ => return Err(Error::NullPointerException),
                            };
                            let obj = heap.get(this_ref)?;
                            // ⚡ Bolt: Pre-allocate vector capacity to avoid multiple reallocations during push/extend
                            let mut impl_args: Vec<Slot> = Vec::with_capacity(
                                lambda_info.captured_count + callee_args.len() - 1,
                            );
                            for i in 0..lambda_info.captured_count {
                                impl_args.push(obj.fields[i]);
                            }
                            impl_args.extend(callee_args[1..].iter().copied());

                            let _ = registry.ensure_loaded_from(
                                &lambda_info.impl_class,
                                Some(current_class.as_str()),
                                loader,
                            );
                            let impl_class_key = registry.class_key_from_source(
                                &lambda_info.impl_class,
                                Some(current_class.as_str()),
                            );

                            if lambda_info.impl_kind == 6 {
                                // invokeStatic dispatch
                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let instrs = std::sync::Arc::clone(
                                            &ctx.methods[impl_idx].instructions,
                                        );
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, instrs, f)
                                    };
                                    match activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        callee_instructions,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        registry,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    ) {
                                        Ok(()) => {}
                                        Err(Error::JavaException { class_name }) => {
                                            throw_java!(class_name);
                                        }
                                        Err(other) => return Err(other),
                                    }
                                    *idx = 0;
                                    continue;
                                }
                            } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                // invokeVirtual / invokeInterface dispatch
                                let impl_class_key = registry.class_key_from_source(
                                    &lambda_info.impl_class,
                                    Some(current_class.as_str()),
                                );
                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let instrs = std::sync::Arc::clone(
                                            &ctx.methods[impl_idx].instructions,
                                        );
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        let expanded_impl_args = expand_args_for_desc(
                                            &impl_args,
                                            &lambda_info.impl_desc,
                                        );
                                        for (i, slot) in expanded_impl_args.into_iter().enumerate()
                                        {
                                            if i < max_locals {
                                                locals_buf[i] = slot;
                                            }
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, instrs, f)
                                    };
                                    match activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        callee_instructions,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        registry,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    ) {
                                        Ok(()) => {}
                                        Err(Error::JavaException { class_name }) => {
                                            throw_java!(class_name);
                                        }
                                        Err(other) => return Err(other),
                                    }
                                    *idx = 0;
                                    continue;
                                }
                                // Try native fallback for virtual/interface
                                let lambda_native_kind = lookup_registered_native_kind(
                                    registry,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                match lambda_native_kind {
                                    Some(HandlerKind::Simple(handler)) => {
                                        #[cfg(feature = "telemetry")]
                                        let _native_start = std::time::Instant::now();
                                        let mut native_control = native_control_for_call(
                                            registry,
                                            current_class,
                                            *method_idx,
                                            pc,
                                            call_stack,
                                            &impl_class_key,
                                            &lambda_info.impl_method,
                                            &lambda_info.impl_desc,
                                        );
                                        let result =
                                            handler(&impl_args, heap, stdout, &mut native_control);
                                        #[cfg(feature = "telemetry")]
                                        registry.telemetry.native_boundary.record_call(
                                            registry.internal_name_for_class(&impl_class_key),
                                            &lambda_info.impl_method,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        let result = handle_native_result!(
                                            result, registry, loader, heap, pc
                                        );
                                        let result = autobox_if_needed(
                                            result,
                                            &lambda_info.impl_desc,
                                            &lambda_info.sam_desc,
                                            heap,
                                        )?;
                                        if let Some(outcome) = finish_native_call(
                                            &mut native_control,
                                            frame,
                                            idx,
                                            result,
                                            &impl_args,
                                        )? {
                                            return Ok(outcome);
                                        }
                                        continue;
                                    }
                                    Some(HandlerKind::Callback(handler)) => {
                                        #[cfg(feature = "telemetry")]
                                        let _native_start = std::time::Instant::now();
                                        let mut native_control = native_control_for_call(
                                            registry,
                                            current_class,
                                            *method_idx,
                                            pc,
                                            call_stack,
                                            &impl_class_key,
                                            &lambda_info.impl_method,
                                            &lambda_info.impl_desc,
                                        );
                                        let result = {
                                            let mut callback_ops =
                                                InterpreterCallbackOps { registry, loader };
                                            handler(
                                                &impl_args,
                                                heap,
                                                stdout,
                                                &mut native_control,
                                                &mut callback_ops,
                                            )
                                        };
                                        #[cfg(feature = "telemetry")]
                                        registry.telemetry.native_boundary.record_call(
                                            registry.internal_name_for_class(&impl_class_key),
                                            &lambda_info.impl_method,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        let result = handle_native_result!(
                                            result, registry, loader, heap, pc
                                        );
                                        let result = autobox_if_needed(
                                            result,
                                            &lambda_info.impl_desc,
                                            &lambda_info.sam_desc,
                                            heap,
                                        )?;
                                        if let Some(outcome) = finish_native_call(
                                            &mut native_control,
                                            frame,
                                            idx,
                                            result,
                                            &impl_args,
                                        )? {
                                            return Ok(outcome);
                                        }
                                        continue;
                                    }
                                    None => {}
                                }
                            } else if lambda_info.impl_kind == 8 {
                                // REF_newInvokeSpecial — constructor reference (e.g. ArrayList::new)
                                // Allocate a new instance, call <init>, push the result.
                                let field_count =
                                    total_instance_field_count(registry, &impl_class_key);
                                let new_ref = heap.allocate(impl_class_key.clone(), field_count);
                                init_object_fields(registry, heap, new_ref, &impl_class_key);
                                let mut init_args = vec![Slot::Reference(Some(new_ref))];
                                init_args.extend_from_slice(&impl_args);
                                match execute_class(
                                    registry,
                                    loader,
                                    heap,
                                    stdout,
                                    &impl_class_key,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                    &init_args,
                                ) {
                                    Ok(_) => {}
                                    Err(Error::JavaException { class_name }) => {
                                        throw_java!(class_name);
                                    }
                                    Err(other) => return Err(other),
                                }
                                if gc_allowed && heap.should_gc() {
                                    let roots = gather_roots(frame, call_stack, registry);
                                    heap.collect(&roots);
                                    patch_forwarded_slots(frame, call_stack, registry, heap);
                                }
                                frame.push(Slot::Reference(Some(new_ref)))?;
                                *idx += 1;
                                continue;
                            }
                            *idx += 1;
                            continue;
                        }
                        if !registry.contains(&actual_class)
                            && !registry.contains(&callee_class_key)
                        {
                            *idx += 1;
                            continue;
                        }
                        return Err(Error::MethodNotFound {
                            name: format!("{actual_class}.{callee_name}"),
                            descriptor: callee_desc,
                        });
                    };

                // Bytecode execution path — Pattern B: pop directly into locals_buf.
                #[cfg(feature = "telemetry")]
                registry.telemetry.dispatch_resolution.record(
                    current_class,
                    cp_idx.0,
                    &dispatch_class,
                    dispatch_class != actual_class,
                );
                let (callee_pc_to_idx, callee_instructions, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let instrs = std::sync::Arc::clone(&ctx.methods[callee_idx].instructions);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(Error::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop method args using wide-type-aware indexing (doubles/longs occupy 2 local slots).
                    pop_typed_args_into_locals(
                        &parse_arg_types(&callee_desc),
                        frame,
                        &mut locals_buf,
                        1,
                    )?;
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, instrs, f)
                };
                match activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    callee_instructions,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    registry,
                    #[cfg(feature = "telemetry")]
                    current_method,
                ) {
                    Ok(()) => {}
                    Err(Error::JavaException { class_name }) => {
                        throw_java!(class_name);
                    }
                    Err(other) => return Err(other),
                }
                *idx = 0;
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
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // ⚡ Bolt: Pre-allocate vector capacity to avoid intermediate reallocations
                // Pop dimension sizes from stack (first popped is rightmost dimension).
                let mut dims: Vec<i32> = Vec::with_capacity(*dimensions as usize);
                for _ in 0..*dimensions {
                    dims.push(frame.pop_int()?);
                }
                dims.reverse(); // Now dims[0] is outermost.

                // Check for negative sizes.
                for &d in &dims {
                    if d < 0 {
                        return Err(Error::NegativeArraySize { size: d });
                    }
                }

                // Recursive allocation helper.
                fn alloc_multi(
                    heap: &mut duke_gc::Heap,
                    dims: &[i32],
                    depth: usize,
                    type_name: &str,
                ) -> Result<u64> {
                    let size = dims[depth] as usize;
                    let r = heap.allocate(type_name.to_string(), size);
                    if depth < dims.len() - 1 {
                        // Not the innermost — fill with references to sub-arrays.
                        let inner_type = &type_name[1..]; // Strip one '[' for inner dimension.
                        for i in 0..size {
                            let inner = alloc_multi(heap, dims, depth + 1, inner_type)?;
                            heap.get_mut(r)?.fields[i] = Slot::Reference(Some(inner));
                        }
                    }
                    Ok(r)
                }

                let r = alloc_multi(heap, &dims, 0, &element_type)?;
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(Error::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        #[cfg(feature = "telemetry")]
        {
            let elapsed = _telem_start.elapsed().as_nanos() as u64;
            registry.telemetry.bytecode_cost.record(
                _telem_name,
                current_class,
                current_method,
                _telem_pc,
                elapsed,
            );
        }

        *idx += 1;
    }
}
