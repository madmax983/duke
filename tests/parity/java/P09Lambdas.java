import java.util.*;
import java.util.function.*;
import java.util.stream.*;

public class P09Lambdas {
    interface Tri { int apply(int a, int b, int c); }

    static int applyTwice(IntUnaryOperator op, int x) { return op.applyAsInt(op.applyAsInt(x)); }

    public static void main(String[] args) {
        // basic lambda + capture
        int factor = 3;
        IntUnaryOperator triple = x -> x * factor;
        System.out.println("capture=" + applyTwice(triple, 2));

        // method refs: static, bound, unbound, constructor
        List<String> words = new ArrayList<>(List.of("banana", "apple", "cherry"));
        words.sort(String::compareTo);
        System.out.println("sort=" + words);
        words.replaceAll(String::toUpperCase);
        System.out.println("upper=" + words);
        Supplier<List<String>> sup = ArrayList::new;
        System.out.println("ctor-ref=" + sup.get().getClass().getSimpleName());
        Function<String, Integer> len = String::length;
        System.out.println("unbound=" + len.apply("hello"));

        // custom functional interface
        Tri t = (a, b, c) -> a + b * c;
        System.out.println("tri=" + t.apply(1, 2, 3));

        // streams pipeline (heavily indy/lambda)
        int sum = IntStream.range(0, 100).filter(i -> i % 2 == 0).map(i -> i * i).sum();
        System.out.println("stream=" + sum);
        String joined = Stream.of("a", "b", "c").collect(Collectors.joining("-"));
        System.out.println("joining=" + joined);
        long cnt = words.stream().filter(w -> w.startsWith("A")).count();
        System.out.println("count=" + cnt);
        Optional<String> first = words.stream().findFirst();
        System.out.println("optional=" + first.orElse("?"));

        // comparator chaining via lambdas
        record P(String n, int a) {}
        List<P> ps = new ArrayList<>(List.of(new P("b", 2), new P("a", 3), new P("a", 1)));
        ps.sort(Comparator.comparing(P::n).thenComparingInt(P::a));
        System.out.println("sorted=" + ps);

        // effectively final in loop
        List<Runnable> rs = new ArrayList<>();
        for (int i = 0; i < 3; i++) { int k = i; rs.add(() -> System.out.print(k)); }
        rs.forEach(Runnable::run);
        System.out.println(" <-loop-capture");
    }
}
