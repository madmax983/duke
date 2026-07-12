// Fixture for the unconditional getfield layout-coherence bounds guard
// (crates/duke-interpreter/src/execution.rs, Getfield arm).
//
// `OobHolder` declares three instance fields, so `OobHolder.c` resolves to slot 2.
// A test allocates an `OobHolder` heap object with FEWER slots than that (mimicking a
// half-migrated object graph — a synthetic native minting an under-sized instance that
// real bytecode then indexes at its full layout) and invokes `readC`. The `getfield`
// on `OobHolder.c` must surface a graceful layout-coherence error, never a raw Rust
// `index out of bounds` panic, even with DUKE_LAYOUT_CHECK unset.
class OobHolder {
    int a;
    int b;
    int c;
}

public class OobFieldProbe {
    static int readC(OobHolder h) {
        return h.c;
    }
}
