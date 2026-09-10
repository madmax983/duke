import java.lang.reflect.RecordComponent;
import java.util.List;

public class P01Records {
    record Point(int x, int y) {
        static int instances = 0;
        Point { // compact constructor: validation + normalization
            if (x < 0 || y < 0) throw new IllegalArgumentException("neg");
            instances++;
        }
        int sum() { return x + y; }
        static Point origin() { return new Point(0, 0); }
    }

    record Pair<T>(T first, T second) {}
    record Wrapper(Point p, String tag) {}

    public static void main(String[] args) {
        Point a = new Point(3, 4);
        Point b = new Point(3, 4);
        Point c = new Point(1, 2);
        System.out.println("accessor=" + a.x() + "," + a.y());
        System.out.println("sum=" + a.sum());
        System.out.println("tostring=" + a);
        System.out.println("equals=" + a.equals(b) + "," + a.equals(c));
        System.out.println("hash=" + (a.hashCode() == b.hashCode()) + "," + (a.hashCode() != c.hashCode()));
        System.out.println("static=" + Point.instances + "," + Point.origin());
        System.out.println("isRecord=" + Point.class.isRecord());
        StringBuilder comps = new StringBuilder();
        for (RecordComponent rc : Point.class.getRecordComponents()) {
            comps.append(rc.getName()).append(":").append(rc.getType().getSimpleName()).append(";");
        }
        System.out.println("components=" + comps);
        try {
            java.lang.reflect.Method acc = null;
            for (RecordComponent rc : Point.class.getRecordComponents()) {
                if (rc.getName().equals("x")) acc = rc.getAccessor();
            }
            System.out.println("reflective-accessor=" + acc.invoke(a));
        } catch (Exception e) {
            System.out.println("reflective-accessor=FAIL:" + e);
        }
        try {
            new Point(-1, 5);
            System.out.println("compact-ctor=NO-THROW");
        } catch (IllegalArgumentException e) {
            System.out.println("compact-ctor=threw");
        }
        Pair<String> p = new Pair<>("a", "b");
        System.out.println("generic=" + p);
        Wrapper w = new Wrapper(new Point(9, 9), "t");
        System.out.println("nested=" + w.p().x() + "," + w.tag());
        // record pattern in instanceof
        Object o = new Point(5, 6);
        if (o instanceof Point(int px, int py)) {
            System.out.println("record-pattern=" + px + "," + py);
        }
        // local record
        record Local(int v) {}
        System.out.println("local-record=" + new Local(42));
        System.out.println("list-of-records=" + List.of(new Point(1, 1), new Point(2, 2)));
    }
}
