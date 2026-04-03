public class IntegerBitOpsTest {
    static int testBitCount() {
        return Integer.bitCount(255);  // 8
    }

    static int testBitCountZero() {
        return Integer.bitCount(0);  // 0
    }

    static int testLeadingZeros() {
        return Integer.numberOfLeadingZeros(1);  // 31
    }

    static int testTrailingZeros() {
        return Integer.numberOfTrailingZeros(8);  // 3
    }

    static int testHighestOneBit() {
        return Integer.highestOneBit(100);  // 64
    }

    static int testLowestOneBit() {
        return Integer.lowestOneBit(12);  // 4
    }

    static int testSignumPositive() {
        return Integer.signum(42);  // 1
    }

    static int testSignumNegative() {
        return Integer.signum(-7);  // -1
    }

    static int testSignumZero() {
        return Integer.signum(0);  // 0
    }

    static int testCompare() {
        return Integer.compare(5, 3);  // 1
    }

    static int testSum() {
        return Integer.sum(7, 13);  // 20
    }

    static int testMax() {
        return Integer.max(3, 9);  // 9
    }

    static int testMin() {
        return Integer.min(3, 9);  // 3
    }
}
