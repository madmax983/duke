public class ParseArgs {
    public static int parseInt(String[] args) {
        return Integer.parseInt(args[0]);
    }

    public static int addParsed(String[] args) {
        int a = Integer.parseInt(args[0]);
        int b = Integer.parseInt(args[1]);
        return a + b;
    }

    public static int valueOf() {
        Integer i = Integer.valueOf(42);
        return i.intValue();
    }

    public static int mathMax() {
        return Math.max(3, 7);
    }

    public static int mathMin() {
        return Math.min(3, 7);
    }

    public static int mathAbs() {
        return Math.abs(-5);
    }
}
