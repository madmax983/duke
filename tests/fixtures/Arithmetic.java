public class Arithmetic {
    public static int add(int a, int b)       { return a + b; }
    public static int subtract(int a, int b)  { return a - b; }
    public static int multiply(int a, int b)  { return a * b; }
    public static int divide(int a, int b)    { return a / b; }
    public static int remainder(int a, int b) { return a % b; }
    public static int negate(int n)           { return -n; }
    public static int shiftLeft(int n, int s) { return n << s; }
    public static int bitwiseAnd(int a, int b){ return a & b; }
    public static int bitwiseOr(int a, int b) { return a | b; }
    public static int bitwiseXor(int a, int b){ return a ^ b; }

    public static int max(int a, int b) {
        return a > b ? a : b;
    }

    public static int abs(int n) {
        return n < 0 ? -n : n;
    }

    public static int clamp(int v, int lo, int hi) {
        if (v < lo) return lo;
        if (v > hi) return hi;
        return v;
    }

    public static int factorial(int n) {
        int result = 1;
        for (int i = 2; i <= n; i++) {
            result *= i;
        }
        return result;
    }

    public static int fibonacci(int n) {
        if (n <= 1) return n;
        int a = 0, b = 1;
        for (int i = 2; i <= n; i++) {
            int t = a + b;
            a = b;
            b = t;
        }
        return b;
    }

    public static int sumTo(int n) {
        int sum = 0;
        for (int i = 1; i <= n; i++) {
            sum += i;
        }
        return sum;
    }

    public static long addLong(long a, long b)         { return a + b; }
    public static double addDouble(double a, double b) { return a + b; }
}
