// Regression fixture for the nested-<clinit> GC root bug.
//
// `run()` allocates an int[] and keeps its ONLY reference in a local slot, then
// triggers initialization of NestedClinitGcInner. That <clinit> runs in a nested
// interpreter execution and allocates enough to force a garbage collection while
// this frame is suspended on the Rust stack. If the collector fails to treat
// this suspended frame as a GC root, the guarded array is reclaimed while still
// young and its young-gen index is reused by the <clinit>'s own allocations, so
// `guarded[0]` no longer reads back the sentinel.
public class NestedClinitGcOuter {
    public static int run() {
        int[] guarded = new int[] { 1234567 };
        // Triggers NestedClinitGcInner.<clinit> (nested execution + GC).
        int trigger = NestedClinitGcInner.VALUE;
        // If `guarded` survived intact this is 1234567 + 499500 = 1734067.
        return guarded[0] + trigger;
    }
}

class NestedClinitGcInner {
    static int VALUE;

    static {
        int acc = 0;
        // Allocate many short-lived arrays. The default collector fires once
        // ~256 allocations accumulate, so this reliably triggers at least one
        // collection during this <clinit> (a nested execution) and then keeps
        // allocating afterwards, reusing the freed young-gen indices.
        for (int i = 0; i < 1000; i++) {
            int[] tmp = new int[8];
            tmp[0] = i;
            acc += tmp[0];
        }
        VALUE = acc; // 0 + 1 + ... + 999 = 499500
    }
}
