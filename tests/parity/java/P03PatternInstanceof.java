public class P03PatternInstanceof {
    record Point(int x, int y) {}

    static String classify(Object o) {
        if (o instanceof String s) return "string:" + s.length();
        if (o instanceof Point(int x, int y)) return "point:" + (x + y);
        if (o instanceof Integer i && i > 10) return "big-int:" + i;
        if (o instanceof Integer i) return "int:" + i;
        if (o == null) return "null";
        return "other:" + o.getClass().getSimpleName();
    }

    public static void main(String[] args) {
        System.out.println(classify("hello"));
        System.out.println(classify(new Point(3, 4)));
        System.out.println(classify(42));
        System.out.println(classify(5));
        System.out.println(classify(null));
        System.out.println(classify(3.14));

        // pattern variable flow scoping
        Object o = "abc";
        boolean b = o instanceof String s && s.length() == 3;
        System.out.println("flow=" + b);

        // negated flow
        if (!(o instanceof String t)) {
            System.out.println("neg=unreachable");
        } else {
            System.out.println("neg=" + t.toUpperCase());
        }

        // instanceof with generic record pattern
        record Box<T>(T v) {}
        Object bo = new Box<>("zzz");
        if (bo instanceof Box(String v)) {
            System.out.println("generic-record-pattern=" + v);
        }

        // nested record patterns
        record Line(Point a, Point b) {}
        Object lo = new Line(new Point(1, 2), new Point(3, 4));
        if (lo instanceof Line(Point(int x1, int y1), Point(int x2, int y2))) {
            System.out.println("nested=" + x1 + "," + y1 + "," + x2 + "," + y2);
        }
    }
}
