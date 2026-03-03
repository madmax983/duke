public class PrintAll {
    public static int printLong() {
        long x = 9876543210L;
        System.out.println(x);
        return 1;
    }

    public static int printDouble() {
        double d = 3.14;
        System.out.println(d);
        return 1;
    }

    public static int printFloat() {
        float f = 2.5f;
        System.out.println(f);
        return 1;
    }

    public static int printBoolean() {
        System.out.println(true);
        System.out.println(false);
        return 1;
    }

    public static int printChar() {
        char c = 'Z';
        System.out.println(c);
        return 1;
    }

    public static int printMixed() {
        System.out.print("val=");
        System.out.print(42);
        System.out.println();
        return 1;
    }
}
