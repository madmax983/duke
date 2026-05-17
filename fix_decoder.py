import re

with open('crates/duke-bytecode/src/decoder.rs', 'r') as f:
    content = f.read()

# Add pc to decode_constant_op
content = content.replace(
    'fn decode_constant_op(c: &mut Cursor<\'_>, opcode: u8) -> Result<Instruction> {',
    'fn decode_constant_op(c: &mut Cursor<\'_>, opcode: u8, pc: usize) -> Result<Instruction> {'
)

# Add pc to decode_load_store_op
content = content.replace(
    'fn decode_load_store_op(c: &mut Cursor<\'_>, opcode: u8) -> Result<Instruction> {',
    'fn decode_load_store_op(c: &mut Cursor<\'_>, opcode: u8, pc: usize) -> Result<Instruction> {'
)

# Add pc to decode_math_op
content = content.replace(
    'fn decode_math_op(c: &mut Cursor<\'_>, opcode: u8) -> Result<Instruction> {',
    'fn decode_math_op(c: &mut Cursor<\'_>, opcode: u8, pc: usize) -> Result<Instruction> {'
)

# Add pc to decode_conversion_op
content = content.replace(
    'fn decode_conversion_op(opcode: u8) -> Result<Instruction> {',
    'fn decode_conversion_op(opcode: u8, pc: usize) -> Result<Instruction> {'
)

# Add pc to decode_extended_op
content = content.replace(
    'fn decode_extended_op(c: &mut Cursor<\'_>, opcode: u8) -> Result<Instruction> {',
    'fn decode_extended_op(c: &mut Cursor<\'_>, opcode: u8, pc: usize) -> Result<Instruction> {'
)

# Replace unreachable!() with DecodeError
content = re.sub(
    r'_ => unreachable!\(\),',
    r'_ => return Err(crate::Error::Decode(crate::error::DecodeError::UnknownOpcode { pc, opcode })),',
    content
)

# Update decode_one calls
content = content.replace(
    'op::NOP..=op::LDC2_W => decode_constant_op(c, opcode),',
    'op::NOP..=op::LDC2_W => decode_constant_op(c, opcode, pc),'
)
content = content.replace(
    'op::ILOAD..=op::SASTORE => decode_load_store_op(c, opcode),',
    'op::ILOAD..=op::SASTORE => decode_load_store_op(c, opcode, pc),'
)
content = content.replace(
    'op::IADD..=op::IINC => decode_math_op(c, opcode),',
    'op::IADD..=op::IINC => decode_math_op(c, opcode, pc),'
)
content = content.replace(
    'op::I2L..=op::I2S => decode_conversion_op(opcode),',
    'op::I2L..=op::I2S => decode_conversion_op(opcode, pc),'
)
content = content.replace(
    'op::MULTIANEWARRAY..=op::JSR_W => decode_extended_op(c, opcode),',
    'op::MULTIANEWARRAY..=op::JSR_W => decode_extended_op(c, opcode, pc),'
)

with open('crates/duke-bytecode/src/decoder.rs', 'w') as f:
    f.write(content)
