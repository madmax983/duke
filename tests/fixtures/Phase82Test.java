import java.util.*;

public class Phase82Test {

    // ---- 2D arrays ----
    public static int testTwoDimArray() {
        int[][] grid = new int[3][4];
        grid[1][2] = 42;
        return grid[1][2]; // 42
    }

    public static int testTwoDimArraySum() {
        int[][] m = {{1,2,3},{4,5,6},{7,8,9}};
        int sum = 0;
        for (int[] row : m)
            for (int v : row) sum += v;
        return sum; // 45
    }

    public static int testTwoDimArrayLength() {
        int[][] a = new int[3][5];
        return a.length * a[0].length; // 15
    }

    // ---- User-defined Comparable ----
    static class Point implements Comparable<Point> {
        int x, y;
        Point(int x, int y) { this.x = x; this.y = y; }
        public int compareTo(Point other) {
            if (this.x != other.x) return Integer.compare(this.x, other.x);
            return Integer.compare(this.y, other.y);
        }
    }

    public static int testUserComparable() {
        List<Point> pts = new ArrayList<>();
        pts.add(new Point(3, 1));
        pts.add(new Point(1, 5));
        pts.add(new Point(2, 2));
        pts.add(new Point(1, 3));
        Collections.sort(pts);
        // sorted: (1,3), (1,5), (2,2), (3,1) → first is (1,3)
        return pts.get(0).x * 10 + pts.get(0).y; // 13
    }

    // ---- String switch ----
    public static int testStringSwitch() {
        String s = "hello";
        switch (s) {
            case "world": return 1;
            case "hello": return 2;
            case "foo":   return 3;
            default:      return 0;
        }
    }

    public static int testStringSwitchDefault() {
        String s = "bar";
        switch (s) {
            case "foo": return 1;
            case "baz": return 2;
            default:    return 99;
        }
    }

    // ---- Varargs user method ----
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }

    public static int testVarargs() {
        return sum(1, 2, 3, 4, 5); // 15
    }

    public static int testVarargsEmpty() {
        return sum(); // 0
    }

    // ---- instanceof pattern ----
    public static int testInstanceofChain() {
        Object o = "hello";
        if (o instanceof String) {
            return ((String) o).length(); // 5
        }
        return 0;
    }

    // ---- Ternary with autoboxing ----
    public static int testTernary() {
        int x = 5;
        Integer result = x > 3 ? x * 2 : x / 2;
        return result; // 10
    }
}
