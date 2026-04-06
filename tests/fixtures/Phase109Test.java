import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase109Test {

    // ---- String.split with limit ----
    public static int testStringSplitLimit() {
        String s = "a:b:c:d:e";
        String[] parts = s.split(":", 3); // ["a", "b", "c:d:e"]
        return parts.length + parts[2].length(); // 3 + 5 = 8
    }

    // ---- Abstract class ----
    abstract static class Animal {
        String name;
        Animal(String name) { this.name = name; }
        abstract int sound();
        int nameLen() { return name.length(); }
    }

    static class Dog extends Animal {
        Dog(String name) { super(name); }
        public int sound() { return 1; } // woof
    }

    static class Cat extends Animal {
        Cat(String name) { super(name); }
        public int sound() { return 2; } // meow
    }

    public static int testAbstractClass() {
        List<Animal> animals = Arrays.asList(new Dog("Rex"), new Cat("Tom"), new Dog("Buddy"));
        return animals.stream()
            .mapToInt(a -> a.sound() + a.nameLen())
            .sum(); // (1+3)+(2+3)+(1+5) = 4+5+6 = 15
    }

    // ---- Map.putAll ----
    public static int testMapPutAll() {
        Map<String, Integer> src = new HashMap<>();
        src.put("a", 1); src.put("b", 2); src.put("c", 3);
        Map<String, Integer> dst = new HashMap<>();
        dst.put("d", 4);
        dst.putAll(src);
        return dst.size(); // 4
    }

    // ---- Collectors.groupingBy simple ----
    public static int testGroupingBySimple() {
        List<String> words = Arrays.asList("cat", "car", "bat", "bar", "bee");
        Map<Character, List<String>> groups = words.stream()
            .collect(Collectors.groupingBy(s -> s.charAt(0)));
        return groups.get('c').size() + groups.get('b').size(); // 2 + 3 = 5
    }

    // ---- Integer.parseInt with radix ----
    public static int testIntegerParseIntRadix() {
        int hex = Integer.parseInt("FF", 16);   // 255
        int bin = Integer.parseInt("1010", 2);  // 10
        return hex - bin; // 245
    }

    // ---- String.format with %c ----
    public static int testStringFormatChar() {
        char c = 'A';
        String s = String.format("Char: %c", c);
        return s.length(); // "Char: A" = 7
    }

    // ---- List.contains ----
    public static int testListContains() {
        List<String> list = Arrays.asList("apple", "banana", "cherry");
        int result = 0;
        if (list.contains("banana")) result += 1;
        if (!list.contains("grape")) result += 1;
        return result; // 2
    }

    // ---- Stream.peek ----
    public static int testStreamPeek() {
        int[] seen = {0};
        int sum = Stream.of(1, 2, 3, 4, 5)
            .peek(n -> seen[0]++)
            .mapToInt(Integer::intValue)
            .sum();
        return sum + seen[0]; // 15 + 5 = 20
    }

    // ---- Nested static classes ----
    static class Outer {
        static int value = 10;
        static class Inner {
            int multiply(int n) { return value * n; }
        }
    }

    public static int testNestedStaticClass() {
        Outer.Inner inner = new Outer.Inner();
        return inner.multiply(5); // 50
    }

    // ---- Collections.disjoint ----
    public static int testCollectionsDisjoint() {
        List<Integer> a = Arrays.asList(1, 2, 3);
        List<Integer> b = Arrays.asList(4, 5, 6);
        List<Integer> c = Arrays.asList(3, 4, 5);
        return (Collections.disjoint(a, b) ? 1 : 0) + (Collections.disjoint(a, c) ? 0 : 1); // 1+1 = 2
    }
}
