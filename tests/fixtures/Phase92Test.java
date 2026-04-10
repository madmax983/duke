import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase92Test {

    // ---- Varargs method ----
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }

    public static int testVarargs() {
        return sum(1, 2, 3, 4, 5); // 15
    }

    // ---- String.format with multiple args ----
    public static int testStringFormatArgs() {
        String s = String.format("%s has %d items worth $%.2f", "Cart", 3, 9.99);
        return s.length(); // "Cart has 3 items worth $9.99" = 29
    }

    // ---- Chained stream operations ----
    public static int testChainedStream() {
        return IntStream.rangeClosed(1, 10)
            .filter(n -> n % 2 == 0)
            .map(n -> n * n)
            .sum(); // 4+16+36+64+100 = 220
    }

    // ---- HashMap iteration with values() ----
    public static int testHashMapValues() {
        Map<String, Integer> scores = new HashMap<>();
        scores.put("Alice", 90);
        scores.put("Bob", 85);
        scores.put("Carol", 92);
        int total = 0;
        for (int score : scores.values()) {
            total += score;
        }
        return total; // 267
    }

    // ---- String split and process ----
    public static int testStringSplitProcess() {
        String csv = "10,20,30,40,50";
        String[] parts = csv.split(",");
        int sum = 0;
        for (String p : parts) sum += Integer.parseInt(p);
        return sum; // 150
    }

    // ---- do-while loop ----
    public static int testDoWhile() {
        int n = 1;
        int product = 1;
        do {
            product *= n;
            n++;
        } while (n <= 5);
        return product; // 1*2*3*4*5 = 120
    }

    // ---- Labeled break ----
    public static int testLabeledBreak() {
        int count = 0;
        outer:
        for (int i = 0; i < 5; i++) {
            for (int j = 0; j < 5; j++) {
                if (i + j >= 5) break outer;
                count++;
            }
        }
        return count; // 0+1+2+3+4 = 10... actually let me trace:
        // i=0: j=0(0<5 ok),j=1(1<5 ok),j=2(2<5 ok),j=3(3<5 ok),j=4(4<5 ok) → 5 counts
        // i=1: j=0(1<5 ok),j=1(2<5 ok),j=2(3<5 ok),j=3(4<5 ok),j=4(5>=5 break outer)
        // total = 5 + 4 = 9... wait no, break outer happens before count++? Let me re-examine:
        // j=4,i=1: check 1+4=5 >= 5, break outer. count is 9. Yes.
    }

    // ---- Array of objects ----
    static class Point {
        int x, y;
        Point(int x, int y) { this.x = x; this.y = y; }
        int dist() { return Math.abs(x) + Math.abs(y); }
    }

    public static int testArrayOfObjects() {
        Point[] pts = { new Point(1, 2), new Point(-3, 4), new Point(0, 5) };
        int total = 0;
        for (Point p : pts) total += p.dist();
        return total; // 3 + 7 + 5 = 15
    }

    // ---- Collector.groupingBy ----
    public static int testGroupingBy() {
        List<String> words = Arrays.asList("one", "two", "three", "four", "five");
        Map<Integer, List<String>> byLen = words.stream()
            .collect(Collectors.groupingBy(String::length));
        // len 3: [one, two], len 4: [four, five], len 5: [three]
        return byLen.get(3).size() + byLen.get(5).size(); // 2 + 1 = 3
    }

    // ---- String.format %c ----
    public static int testStringFormatChar() {
        String s = String.format("Char: %c, Num: %d", 'A', 65);
        return s.length(); // "Char: A, Num: 65" = 16
    }
}
