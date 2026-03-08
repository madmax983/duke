import java.util.ArrayList;
import java.util.HashMap;

public class BenchmarkSuite {

    // Arithmetic loop: sum 0..N-1 (wraps silently in int)
    static int benchSum() {
        int sum = 0;
        for (int i = 0; i < 500_000; i++) sum += i;
        return sum;  // 445698416 (124,999,750,000 wraps in i32)
    }

    // Recursive fibonacci — exercises method dispatch
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }
    static int benchFib() {
        return fib(25);  // 75025; ~500k recursive calls
    }

    // ArrayList: add N boxed ints, return size
    static int benchArrayList() {
        ArrayList<Integer> list = new ArrayList<>();
        for (int i = 0; i < 5_000; i++) {
            list.add(Integer.valueOf(i));
        }
        return list.size();  // 5000
    }

    // HashMap: put 200 string-keyed entries, then get all, return checksum
    static int benchHashMap() {
        HashMap<String, Integer> map = new HashMap<>();
        for (int i = 0; i < 200; i++) {
            map.put("key" + i, Integer.valueOf(i));
        }
        int sum = 0;
        for (int i = 0; i < 200; i++) {
            Integer v = (Integer) map.get("key" + i);
            if (v != null) sum += v.intValue();
        }
        return sum;  // 0+1+..+199 = 199*200/2 = 19900
    }

    // main: HotSpot comparison — takes benchmark name as arg, runs it once
    public static void main(String[] args) {
        String name = args.length > 0 ? args[0] : "";
        int result;
        if ("sum".equals(name)) {
            result = benchSum();
        } else if ("fib".equals(name)) {
            result = benchFib();
        } else if ("arraylist".equals(name)) {
            result = benchArrayList();
        } else if ("hashmap".equals(name)) {
            result = benchHashMap();
        } else {
            // default: run all, print hashmap checksum as final result
            benchSum(); benchFib(); benchArrayList();
            result = benchHashMap();
        }
        // Print result as sanity check
        System.out.println(result);
    }
}
