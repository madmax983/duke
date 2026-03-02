/**
 * Phase 10 fixture — a class with instance fields, constructor, and methods.
 * Used by PairUser to test cross-class new/invokespecial/invokevirtual.
 */
public class Pair {
    private int x;
    private int y;

    public Pair(int x, int y) {
        this.x = x;
        this.y = y;
    }

    public int sum() {
        return x + y;
    }

    public int diff() {
        return x - y;
    }
}
