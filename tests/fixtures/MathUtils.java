public class MathUtils {
    public static int square(int n) {
        return n * n;
    }

    public static int sumOfSquares(int a, int b) {
        return square(a) + square(b);
    }

    public static int power(int base, int exp) {
        int result = 1;
        while (exp > 0) {
            result = result * base;
            exp = exp - 1;
        }
        return result;
    }

    public static int gcd(int a, int b) {
        while (b != 0) {
            int t = b;
            b = a % b;
            a = t;
        }
        return a;
    }
}
