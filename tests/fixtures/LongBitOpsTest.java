public class LongBitOpsTest {
    static int testBitCount() {
        return Long.bitCount(255L);  // 8
    }

    static int testLeadingZeros() {
        return Long.numberOfLeadingZeros(1L);  // 63
    }

    static int testTrailingZeros() {
        return Long.numberOfTrailingZeros(8L);  // 3
    }

    static int testHighestOneBit() {
        return (int) Long.highestOneBit(100L);  // 64
    }

    static int testLowestOneBit() {
        return (int) Long.lowestOneBit(12L);  // 4
    }

    static int testSignumPositive() {
        return Long.signum(42L);  // 1
    }

    static int testSignumNegative() {
        return Long.signum(-7L);  // -1
    }

    static int testSignumZero() {
        return Long.signum(0L);  // 0
    }

    static int testCompare() {
        return Long.compare(5L, 3L);  // 1
    }

    static int testSum() {
        return (int) Long.sum(7L, 13L);  // 20
    }
}
