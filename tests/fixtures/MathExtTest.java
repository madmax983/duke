public class MathExtTest {
    static int testSin() {
        double result = Math.sin(Math.PI / 2.0);
        return (int) Math.round(result);  // 1
    }

    static int testCos() {
        double result = Math.cos(0.0);
        return (int) Math.round(result);  // 1
    }

    static int testTan() {
        double result = Math.tan(Math.PI / 4.0);
        // tan(π/4) ≈ 1.0
        return (int) Math.round(result);  // 1
    }

    static int testAtan2() {
        double result = Math.atan2(1.0, 1.0);  // π/4
        // atan2(1,1) * 4 / π ≈ 1.0
        return (int) Math.round(result * 4.0 / Math.PI);  // 1
    }

    static int testLog() {
        double result = Math.log(Math.E);  // ln(e) = 1.0
        return (int) Math.round(result);  // 1
    }

    static int testLog10() {
        double result = Math.log10(1000.0);  // log10(1000) = 3.0
        return (int) Math.round(result);  // 3
    }

    static int testExp() {
        double result = Math.exp(0.0);  // e^0 = 1.0
        return (int) Math.round(result);  // 1
    }

    static int testSignumPositive() {
        double result = Math.signum(42.0);
        return (int) result;  // 1
    }

    static int testSignumNegative() {
        double result = Math.signum(-7.5);
        return (int) result;  // -1
    }

    static int testToRadians() {
        double result = Math.toRadians(180.0);
        return (int) Math.round(result / Math.PI);  // 1
    }

    static int testToDegrees() {
        double result = Math.toDegrees(Math.PI);
        return (int) Math.round(result);  // 180
    }

    static int testCbrt() {
        double result = Math.cbrt(27.0);
        return (int) Math.round(result);  // 3
    }

    static int testHypot() {
        double result = Math.hypot(3.0, 4.0);  // sqrt(9+16) = 5
        return (int) Math.round(result);  // 5
    }

    static int testRoundFloat() {
        float f = 2.7f;
        return Math.round(f);  // 3
    }
}
