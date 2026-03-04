public class ExtendedMath {
    public static int testSqrt() {
        double r = Math.sqrt(16.0);
        return r == 4.0 ? 1 : 0;
    }
    public static int testPow() {
        double r = Math.pow(2.0, 10.0);
        return r == 1024.0 ? 1 : 0;
    }
    public static int testFloorCeil() {
        double f = Math.floor(3.7);
        double c = Math.ceil(3.2);
        return (f == 3.0 && c == 4.0) ? 1 : 0;
    }
    public static long testRound() {
        return Math.round(3.6);
    }
    public static int testAbsLong() {
        long r = Math.abs(-42L);
        return r == 42L ? 1 : 0;
    }
    public static int testAbsDouble() {
        double r = Math.abs(-3.14);
        return r == 3.14 ? 1 : 0;
    }
    public static long testMaxLong() {
        return Math.max(100L, 200L);
    }
    public static long testMinLong() {
        return Math.min(100L, 200L);
    }
    public static int testMaxDouble() {
        double r = Math.max(1.5, 2.5);
        return r == 2.5 ? 1 : 0;
    }
    public static long testParseLong() {
        return Long.parseLong("9876543210");
    }
    public static int testParseDouble() {
        double d = Double.parseDouble("3.14");
        return (d > 3.13 && d < 3.15) ? 1 : 0;
    }
    public static int testParseFloat() {
        float f = Float.parseFloat("2.5");
        return f == 2.5f ? 1 : 0;
    }
    public static int testParseBoolean() {
        boolean t = Boolean.parseBoolean("true");
        boolean f = Boolean.parseBoolean("false");
        boolean x = Boolean.parseBoolean("yes");
        return (t && !f && !x) ? 1 : 0;
    }
    public static int testLongValueOf() {
        Long boxed = Long.valueOf(42L);
        long val = boxed.longValue();
        return val == 42L ? 1 : 0;
    }
    public static int testMathConstants() {
        double pi = Math.PI;
        double e = Math.E;
        return (pi > 3.14 && pi < 3.15 && e > 2.71 && e < 2.72) ? 1 : 0;
    }
}
