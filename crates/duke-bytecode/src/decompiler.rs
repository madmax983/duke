//! Experimental Bytecode to Pseudo-Java Decompiler.
//!
//! This module provides a rough translation from basic blocks of JVM bytecode
//! back into readable pseudo-Java code. It tracks stack states to combine
//! operations (e.g., iload + iload + iadd -> a + b).

#[cfg(feature = "nova")]
use crate::{BasicBlock, Instruction};

/// Decompiles a series of basic blocks into pseudo-Java string representation.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::{Instruction, BasicBlock};
/// use duke_bytecode::decompile;
///
/// let blocks = vec![
///     BasicBlock {
///         start_pc: 0,
///         end_pc: 4,
///         instructions: vec![
///             (0, Instruction::Iload(1)),
///             (1, Instruction::Iload(2)),
///             (2, Instruction::Iadd),
///             (3, Instruction::Ireturn),
///         ],
///     }
/// ];
///
/// let code = decompile(&blocks);
/// assert!(code.contains("return (v1 + v2);"));
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn decompile(blocks: &[BasicBlock]) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    for block in blocks {
        let _ = writeln!(out, "L{}:", block.start_pc);
        let mut stack: Vec<String> = Vec::new();

        for (_pc, instr) in &block.instructions {
            match instr {
                Instruction::Nop => {}
                Instruction::Iconst0 => stack.push("0".to_string()),
                Instruction::Iconst1 => stack.push("1".to_string()),
                Instruction::Iconst2 => stack.push("2".to_string()),
                Instruction::Iconst3 => stack.push("3".to_string()),
                Instruction::Iconst4 => stack.push("4".to_string()),
                Instruction::Iconst5 => stack.push("5".to_string()),
                Instruction::IconstM1 => stack.push("-1".to_string()),
                Instruction::Bipush(v) => stack.push(v.to_string()),
                Instruction::Sipush(v) => stack.push(v.to_string()),
                Instruction::AconstNull => stack.push("null".to_string()),

                Instruction::Iload(idx)
                | Instruction::Fload(idx)
                | Instruction::Aload(idx)
                | Instruction::Lload(idx)
                | Instruction::Dload(idx) => {
                    stack.push(format!("v{idx}"));
                }

                Instruction::Iload0
                | Instruction::Fload0
                | Instruction::Aload0
                | Instruction::Lload0
                | Instruction::Dload0 => stack.push("v0".to_string()),
                Instruction::Iload1
                | Instruction::Fload1
                | Instruction::Aload1
                | Instruction::Lload1
                | Instruction::Dload1 => stack.push("v1".to_string()),
                Instruction::Iload2
                | Instruction::Fload2
                | Instruction::Aload2
                | Instruction::Lload2
                | Instruction::Dload2 => stack.push("v2".to_string()),
                Instruction::Iload3
                | Instruction::Fload3
                | Instruction::Aload3
                | Instruction::Lload3
                | Instruction::Dload3 => stack.push("v3".to_string()),

                Instruction::Istore(idx)
                | Instruction::Fstore(idx)
                | Instruction::Astore(idx)
                | Instruction::Lstore(idx)
                | Instruction::Dstore(idx) => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    v{idx} = {val};");
                    }
                }

                Instruction::Istore0
                | Instruction::Fstore0
                | Instruction::Astore0
                | Instruction::Lstore0
                | Instruction::Dstore0 => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    v0 = {val};");
                    }
                }
                Instruction::Istore1
                | Instruction::Fstore1
                | Instruction::Astore1
                | Instruction::Lstore1
                | Instruction::Dstore1 => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    v1 = {val};");
                    }
                }
                Instruction::Istore2
                | Instruction::Fstore2
                | Instruction::Astore2
                | Instruction::Lstore2
                | Instruction::Dstore2 => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    v2 = {val};");
                    }
                }
                Instruction::Istore3
                | Instruction::Fstore3
                | Instruction::Astore3
                | Instruction::Lstore3
                | Instruction::Dstore3 => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    v3 = {val};");
                    }
                }

                Instruction::Iadd | Instruction::Fadd | Instruction::Ladd | Instruction::Dadd => {
                    let b = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let op_str = match instr {
                        Instruction::Iadd | Instruction::Fadd | Instruction::Ladd | Instruction::Dadd => "+",
                        Instruction::Isub | Instruction::Fsub | Instruction::Lsub | Instruction::Dsub => "-",
                        Instruction::Imul | Instruction::Fmul | Instruction::Lmul | Instruction::Dmul => "*",
                        Instruction::Idiv | Instruction::Fdiv | Instruction::Ldiv | Instruction::Ddiv => "/",
                        _ => "?",
                    };
                    stack.push(format!("({a} {op_str} {b})"));
                }
                Instruction::Isub | Instruction::Fsub | Instruction::Lsub | Instruction::Dsub => {
                    let b = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let op_str = match instr {
                        Instruction::Iadd | Instruction::Fadd | Instruction::Ladd | Instruction::Dadd => "+",
                        Instruction::Isub | Instruction::Fsub | Instruction::Lsub | Instruction::Dsub => "-",
                        Instruction::Imul | Instruction::Fmul | Instruction::Lmul | Instruction::Dmul => "*",
                        Instruction::Idiv | Instruction::Fdiv | Instruction::Ldiv | Instruction::Ddiv => "/",
                        _ => "?",
                    };
                    stack.push(format!("({a} {op_str} {b})"));
                }
                Instruction::Imul | Instruction::Fmul | Instruction::Lmul | Instruction::Dmul => {
                    let b = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let op_str = match instr {
                        Instruction::Iadd | Instruction::Fadd | Instruction::Ladd | Instruction::Dadd => "+",
                        Instruction::Isub | Instruction::Fsub | Instruction::Lsub | Instruction::Dsub => "-",
                        Instruction::Imul | Instruction::Fmul | Instruction::Lmul | Instruction::Dmul => "*",
                        Instruction::Idiv | Instruction::Fdiv | Instruction::Ldiv | Instruction::Ddiv => "/",
                        _ => "?",
                    };
                    stack.push(format!("({a} {op_str} {b})"));
                }
                Instruction::Idiv | Instruction::Fdiv | Instruction::Ldiv | Instruction::Ddiv => {
                    let b = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let op_str = match instr {
                        Instruction::Iadd | Instruction::Fadd | Instruction::Ladd | Instruction::Dadd => "+",
                        Instruction::Isub | Instruction::Fsub | Instruction::Lsub | Instruction::Dsub => "-",
                        Instruction::Imul | Instruction::Fmul | Instruction::Lmul | Instruction::Dmul => "*",
                        Instruction::Idiv | Instruction::Fdiv | Instruction::Ldiv | Instruction::Ddiv => "/",
                        _ => "?",
                    };
                    stack.push(format!("({a} {op_str} {b})"));
                }

                Instruction::Ireturn
                | Instruction::Freturn
                | Instruction::Areturn
                | Instruction::Lreturn
                | Instruction::Dreturn => {
                    if let Some(val) = stack.pop() {
                        let _ = writeln!(out, "    return {val};");
                    } else {
                        let _ = writeln!(out, "    return <empty_stack>;");
                    }
                }
                Instruction::Return => {
                    let _ = writeln!(out, "    return;");
                }

                Instruction::Ifeq(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} == 0) goto L{tgt};");
                    }
                }
                Instruction::Ifne(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} != 0) goto L{tgt};");
                    }
                }
                Instruction::Iflt(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} < 0) goto L{tgt};");
                    }
                }
                Instruction::Ifge(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} >= 0) goto L{tgt};");
                    }
                }
                Instruction::Ifgt(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} > 0) goto L{tgt};");
                    }
                }
                Instruction::Ifle(tgt) => {
                    if let Some(a) = stack.pop() {
                        let _ = writeln!(out, "    if ({a} <= 0) goto L{tgt};");
                    }
                }

                Instruction::IfIcmpeq(tgt) | Instruction::IfAcmpeq(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} == {b}) goto L{tgt};");
                    }
                }
                Instruction::IfIcmpne(tgt) | Instruction::IfAcmpne(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} != {b}) goto L{tgt};");
                    }
                }
                Instruction::IfIcmplt(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} < {b}) goto L{tgt};");
                    }
                }
                Instruction::IfIcmpge(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} >= {b}) goto L{tgt};");
                    }
                }
                Instruction::IfIcmpgt(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} > {b}) goto L{tgt};");
                    }
                }
                Instruction::IfIcmple(tgt) => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let _ = writeln!(out, "    if ({a} <= {b}) goto L{tgt};");
                    }
                }

                Instruction::Goto(tgt) => {
                    let _ = writeln!(out, "    goto L{tgt};");
                }

                _ => {
                    // Fallback for not-yet-supported instructions
                    let _ = writeln!(out, "    /* {} */", instr.mnemonic());
                    // Flush stack to avoid misalignment
                    for val in std::mem::take(&mut stack) {
                        let _ = writeln!(out, "    /* pending stack: {val} */");
                    }
                }
            }
        }

        for val in stack {
            let _ = writeln!(out, "    /* unused stack: {val} */");
        }
    }

    out
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::{BasicBlock, Instruction};

    #[test]
    fn test_decompile_arithmetic() {
        let blocks = vec![BasicBlock {
            start_pc: 0,
            end_pc: 4,
            instructions: vec![
                (0, Instruction::Iload1),
                (1, Instruction::Iload2),
                (2, Instruction::Iadd),
                (3, Instruction::Ireturn),
            ],
        }];

        let code = decompile(&blocks);
        assert!(code.contains("return (v1 + v2);"));
    }

    #[test]
    fn test_decompile_branches() {
        let blocks = vec![BasicBlock {
            start_pc: 0,
            end_pc: 3,
            instructions: vec![
                (0, Instruction::Iload1),
                (1, Instruction::Iload2),
                (2, Instruction::IfIcmpeq(10)),
            ],
        }];

        let code = decompile(&blocks);
        assert!(code.contains("if (v1 == v2) goto L10;"));
    }
}
