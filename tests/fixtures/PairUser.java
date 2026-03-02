/**
 * Phase 10 fixture — creates Pair objects and calls their methods.
 * Tests cross-class new, invokespecial (init), and invokevirtual.
 */
public class PairUser {
    /** Create a Pair and return its sum. */
    public static int makePairSum(int x, int y) {
        Pair p = new Pair(x, y);
        return p.sum();
    }

    /** Create a Pair and return its diff. */
    public static int makePairDiff(int x, int y) {
        Pair p = new Pair(x, y);
        return p.diff();
    }

    /** Create two Pairs and return the sum of their sums. */
    public static int twoPairsSum(int a, int b, int c, int d) {
        Pair p1 = new Pair(a, b);
        Pair p2 = new Pair(c, d);
        return p1.sum() + p2.sum();
    }
}
