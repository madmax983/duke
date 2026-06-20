import re

with open("crates/duke-bytecode/src/decompiler.rs", "r") as f:
    content = f.read()

# Istore
content = re.sub(r'Instruction::Istore\((idx)\) \| Instruction::Fstore\((idx)\) \| \n                Instruction::Astore\((idx)\) \| Instruction::Lstore\((idx)\) \| Instruction::Dstore\((idx)\) => \{\n                    if let Some\(val\) = stack.pop\(\) \{\n                        let _ = writeln!\(out, "    v\{idx\} = \{val\};"\);\n                    \}\n                \}',
                 r'''Instruction::Istore(idx) | Instruction::Fstore(idx) |
                Instruction::Astore(idx) | Instruction::Lstore(idx) | Instruction::Dstore(idx) => {
                    let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let _ = writeln!(out, "    v{idx} = {val};");
                }''', content)

# Istore 0-3
content = content.replace('if let Some(val) = stack.pop() { let _ = writeln!(out, "    v0 = {val};"); }', 'let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    v0 = {val};");')
content = content.replace('if let Some(val) = stack.pop() { let _ = writeln!(out, "    v1 = {val};"); }', 'let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    v1 = {val};");')
content = content.replace('if let Some(val) = stack.pop() { let _ = writeln!(out, "    v2 = {val};"); }', 'let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    v2 = {val};");')
content = content.replace('if let Some(val) = stack.pop() { let _ = writeln!(out, "    v3 = {val};"); }', 'let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    v3 = {val};");')

# Arithmetic
def replace_arithmetic(op):
    return re.sub(
        r'Instruction::I' + op + r' \| Instruction::F' + op + r' \| Instruction::L' + op + r' \| Instruction::D' + op + r' => \{\n                    if let \(Some\(b\), Some\(a\)\) = \(stack.pop\(\), stack.pop\(\)\) \{\n                        stack.push\(format!\("\(\{a\} . \{b\}\)"\)\);\n                    \}\n                \}',
        r'''Instruction::I''' + op + r''' | Instruction::F''' + op + r''' | Instruction::L''' + op + r''' | Instruction::D''' + op + r''' => {
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
                }''',
        content
    )

content = replace_arithmetic("add")
content = replace_arithmetic("sub")
content = replace_arithmetic("mul")
content = replace_arithmetic("div")

# Return
content = re.sub(
    r'Instruction::Ireturn \| Instruction::Freturn \| Instruction::Areturn \| Instruction::Lreturn \| Instruction::Dreturn => \{\n                    if let Some\(val\) = stack.pop\(\) \{\n                        let _ = writeln!\(out, "    return \{val\};"\);\n                    \} else \{\n                        let _ = writeln!\(out, "    return <empty_stack>;"\);\n                    \}\n                \}',
    r'''Instruction::Ireturn | Instruction::Freturn | Instruction::Areturn | Instruction::Lreturn | Instruction::Dreturn => {
                    let val = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string());
                    let _ = writeln!(out, "    return {val};");
                }''',
    content
)

# Ifeq
for op, sign in [("eq", "=="), ("ne", "!="), ("lt", "<"), ("ge", ">="), ("gt", ">"), ("le", "<=")]:
    content = re.sub(
        r'Instruction::If' + op + r'\(tgt\) => \{ if let Some\(a\) = stack.pop\(\) \{ let _ = writeln!\(out, "    if \(\{a\} ' + sign + r' 0\) goto L\{tgt\};"\); \} \}',
        r'''Instruction::If''' + op + r'''(tgt) => { let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    if ({a} ''' + sign + ''' 0) goto L{tgt};"); }''',
        content
    )

# IfIcmp
for op, sign in [("eq", "=="), ("ne", "!="), ("lt", "<"), ("ge", ">="), ("gt", ">"), ("le", "<=")]:
    pattern = r'Instruction::IfIcmp' + op + r'\(tgt\) \| Instruction::IfAcmp' + op + r'\(tgt\) => \{ if let \(Some\(b\), Some\(a\)\) = \(stack.pop\(\), stack.pop\(\)\) \{ let _ = writeln!\(out, "    if \(\{a\} ' + sign + r' \{b\}\) goto L\{tgt\};"\); \} \}'
    if op not in ("eq", "ne"):
        pattern = r'Instruction::IfIcmp' + op + r'\(tgt\) => \{ if let \(Some\(b\), Some\(a\)\) = \(stack.pop\(\), stack.pop\(\)\) \{ let _ = writeln!\(out, "    if \(\{a\} ' + sign + r' \{b\}\) goto L\{tgt\};"\); \} \}'

    replace = r'''Instruction::IfIcmp''' + op + r'''(tgt)'''
    if op in ("eq", "ne"):
        replace += r''' | Instruction::IfAcmp''' + op + r'''(tgt)'''
    replace += r''' => { let b = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let a = stack.pop().unwrap_or_else(|| "<stack_underflow>".to_string()); let _ = writeln!(out, "    if ({a} ''' + sign + ''' {b}) goto L{tgt};"); }'''

    content = re.sub(pattern, replace, content)

with open("crates/duke-bytecode/src/decompiler.rs", "w") as f:
    f.write(content)
