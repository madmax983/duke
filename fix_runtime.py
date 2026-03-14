with open('crates/duke-runtime/src/frame.rs', 'r') as f:
    data = f.read()

data = data.replace('pub fn from_pool_bufs(', '#[must_use]\n    pub fn from_pool_bufs(')
data = data.replace('pub fn into_pool_bufs(', '#[must_use]\n    pub fn into_pool_bufs(')
data = data.replace('pub fn stack_len(&self)', 'pub const fn stack_len(&self)')

s1 = "/// Peek at the slot at a given absolute position in the operand stack (0-indexed from bottom).\n    pub fn peek_at"
s2 = "/// Peek at the slot at a given absolute position in the operand stack (0-indexed from bottom).\n    ///\n    /// # Errors\n    /// Returns `VmError::StackUnderflow` if the index is out of bounds.\n    pub fn peek_at"
data = data.replace(s1, s2)

with open('crates/duke-runtime/src/frame.rs', 'w') as f:
    f.write(data)
