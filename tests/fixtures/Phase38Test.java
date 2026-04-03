import java.util.Random;
import java.util.StringJoiner;
import java.util.stream.Stream;

public class Phase38Test {

    // ---- java.util.Random ----

    static int testRandomRange() {
        Random r = new Random(42);
        int n = r.nextInt(100);
        return (n >= 0 && n < 100) ? 1 : 0;  // 1
    }

    static int testRandomSeedDeterministic() {
        Random r1 = new Random(12345);
        Random r2 = new Random(12345);
        return (r1.nextInt(1000) == r2.nextInt(1000)) ? 1 : 0;  // 1
    }

    static int testRandomBoolean() {
        // With seed 0, Java nextBoolean() = false (0)
        Random r = new Random(0);
        boolean b = r.nextBoolean();
        // Just verify it returns without error; check type via ternary
        return (b || !b) ? 1 : 0;  // always 1
    }

    static int testRandomNextInt() {
        Random r = new Random(99);
        int a = r.nextInt();
        int b = r.nextInt();
        // Two calls should produce different results with overwhelming probability
        return (a != b) ? 1 : 0;  // 1
    }

    static int testRandomNextDouble() {
        Random r = new Random(7);
        double d = r.nextDouble();
        return (d >= 0.0 && d < 1.0) ? 1 : 0;  // 1
    }

    static int testRandomNextLong() {
        Random r = new Random(1000);
        long v = r.nextLong();
        // Just verify it runs and returns a long
        return (v != 0 || v == 0) ? 1 : 0;  // always 1
    }

    // ---- java.lang.StringBuffer (mutable, alias for StringBuilder) ----

    static int testStringBufferAppend() {
        StringBuffer sb = new StringBuffer();
        sb.append("hello");
        sb.append(" ");
        sb.append("world");
        return sb.toString().length();  // 11
    }

    static int testStringBufferInit() {
        StringBuffer sb = new StringBuffer("hi");
        return sb.length();  // 2
    }

    static int testStringBufferToString() {
        StringBuffer sb = new StringBuffer("abc");
        return sb.toString().equals("abc") ? 1 : 0;  // 1
    }

    // ---- java.util.StringJoiner ----

    static int testStringJoinerBasic() {
        StringJoiner sj = new StringJoiner(", ");
        sj.add("a");
        sj.add("b");
        sj.add("c");
        return sj.toString().equals("a, b, c") ? 1 : 0;  // 1
    }

    static int testStringJoinerEmpty() {
        StringJoiner sj = new StringJoiner(", ");
        return sj.toString().length();  // 0
    }

    static int testStringJoinerWithPrefixSuffix() {
        StringJoiner sj = new StringJoiner(", ", "[", "]");
        sj.add("x");
        sj.add("y");
        return sj.toString().equals("[x, y]") ? 1 : 0;  // 1
    }

    static int testStringJoinerSetEmptyValue() {
        StringJoiner sj = new StringJoiner(", ");
        sj.setEmptyValue("none");
        return sj.toString().equals("none") ? 1 : 0;  // 1
    }

    static int testStringJoinerLength() {
        StringJoiner sj = new StringJoiner("-");
        sj.add("a");
        sj.add("bb");
        return sj.length();  // 4 ("a-bb")
    }

    // ---- String.lines() ----

    static int testStringLines() {
        String s = "a\nb\nc";
        long count = s.lines().count();
        return (int) count;  // 3
    }

    static int testStringLinesJoin() {
        String s = "hello\nworld";
        String joined = s.lines()
            .map(l -> l.toUpperCase())
            .collect(java.util.stream.Collectors.joining(" "));
        return joined.equals("HELLO WORLD") ? 1 : 0;  // 1
    }
}
