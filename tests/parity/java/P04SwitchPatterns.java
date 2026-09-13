public class P04SwitchPatterns {
    record Point(int x, int y) {}
    sealed interface Expr permits Const, Add, Neg {}
    record Const(int v) implements Expr {}
    record Add(Expr l, Expr r) implements Expr {}
    record Neg(Expr e) implements Expr {}

    static int eval(Expr e) {
        return switch (e) {
            case Const(int v) -> v;
            case Add(Expr l, Expr r) -> eval(l) + eval(r);
            case Neg(Expr x) -> -eval(x);
        };
    }

    static String describe(Object o) {
        return switch (o) {
            case null -> "null!";
            case String s when s.isEmpty() -> "empty-string";
            case String s -> "string:" + s;
            case Point(int x, int y) when x == y -> "diag-point";
            case Point p -> "point:" + p.x();
            case Integer i when i < 0 -> "negative";
            case Integer i -> "int:" + i;
            case int[] arr -> "int-array:" + arr.length;
            default -> "other";
        };
    }

    enum Color { RED, GREEN, BLUE }

    public static void main(String[] args) {
        System.out.println(describe(null));
        System.out.println(describe(""));
        System.out.println(describe("hi"));
        System.out.println(describe(new Point(2, 2)));
        System.out.println(describe(new Point(1, 5)));
        System.out.println(describe(-7));
        System.out.println(describe(9));
        System.out.println(describe(new int[3]));
        System.out.println(describe(1.5));

        Expr e = new Add(new Const(2), new Neg(new Const(5)));
        System.out.println("eval=" + eval(e));

        // enum switch still works alongside
        Color col = Color.GREEN;
        String cn = switch (col) {
            case RED -> "r";
            case GREEN -> "g";
            case BLUE -> "b";
        };
        System.out.println("enum=" + cn);

        // fallthrough with pattern switch is a compile error, so we test
        // selector-expression evaluation order instead
        int[] counts = {0};
        Object tricky = new Object() {
            @Override public String toString() { counts[0]++; return "x"; }
        };
        String r = switch (tricky) {
            case String s -> "s";
            default -> "d";
        };
        System.out.println("selector-eval=" + r + "," + counts[0]);
    }
}
