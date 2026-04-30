import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

public class StringBytesRoundTripTest {
    private static final String ASCII = "Hello, World!";
    private static final String LATIN1 = "Caf\u00e9 r\u00e9sum\u00e9 na\u00efvet\u00e9";
    private static final String BMP = "\u4f60\u597d\u4e16\u754c \u2014 \u03b1\u03b2\u03b3 \u2014 \u2699\ufe0f";
    private static final String EMPTY = "";

    private static boolean roundTrip(Charset cs, String value) {
        return new String(value.getBytes(cs), cs).equals(value);
    }

    private static String emoji() {
        byte[] bytes = new byte[] {
            (byte)0xf0, (byte)0x9f, (byte)0x98, (byte)0x80,
            0x20, 0x65, 0x6d, 0x6f, 0x6a, 0x69
        };
        return new String(bytes, StandardCharsets.UTF_8);
    }

    public static int utf8Matrix() {
        return roundTrip(StandardCharsets.UTF_8, ASCII)
            && roundTrip(StandardCharsets.UTF_8, LATIN1)
            && roundTrip(StandardCharsets.UTF_8, BMP)
            && roundTrip(StandardCharsets.UTF_8, emoji())
            && roundTrip(StandardCharsets.UTF_8, EMPTY) ? 1 : 0;
    }

    public static int utf16Matrix() {
        return roundTrip(StandardCharsets.UTF_16, ASCII)
            && roundTrip(StandardCharsets.UTF_16, LATIN1)
            && roundTrip(StandardCharsets.UTF_16, BMP)
            && roundTrip(StandardCharsets.UTF_16, emoji())
            && roundTrip(StandardCharsets.UTF_16, EMPTY) ? 1 : 0;
    }

    public static int utf16beMatrix() {
        return roundTrip(StandardCharsets.UTF_16BE, ASCII)
            && roundTrip(StandardCharsets.UTF_16BE, LATIN1)
            && roundTrip(StandardCharsets.UTF_16BE, BMP)
            && roundTrip(StandardCharsets.UTF_16BE, emoji())
            && roundTrip(StandardCharsets.UTF_16BE, EMPTY) ? 1 : 0;
    }

    public static int utf16leMatrix() {
        return roundTrip(StandardCharsets.UTF_16LE, ASCII)
            && roundTrip(StandardCharsets.UTF_16LE, LATIN1)
            && roundTrip(StandardCharsets.UTF_16LE, BMP)
            && roundTrip(StandardCharsets.UTF_16LE, emoji())
            && roundTrip(StandardCharsets.UTF_16LE, EMPTY) ? 1 : 0;
    }

    public static int latin1InRangeMatrix() {
        return roundTrip(StandardCharsets.ISO_8859_1, ASCII)
            && roundTrip(StandardCharsets.ISO_8859_1, LATIN1)
            && roundTrip(StandardCharsets.ISO_8859_1, EMPTY) ? 1 : 0;
    }

    public static int asciiInRangeMatrix() {
        return roundTrip(StandardCharsets.US_ASCII, ASCII)
            && roundTrip(StandardCharsets.US_ASCII, EMPTY) ? 1 : 0;
    }

    public static int lossyCharsetsUseQuestionMark() {
        boolean ascii = new String("\u20ac".getBytes(StandardCharsets.US_ASCII), StandardCharsets.US_ASCII).equals("?");
        boolean latin1 = new String("\u20ac".getBytes(StandardCharsets.ISO_8859_1), StandardCharsets.ISO_8859_1).equals("?");
        return ascii && latin1 ? 1 : 0;
    }

    public static int knownEncodedLengths() {
        boolean utf8 = "abc".getBytes(StandardCharsets.UTF_8).length == 3;
        boolean utf16 = "abc".getBytes(StandardCharsets.UTF_16).length == 8;
        boolean utf16be = "abc".getBytes(StandardCharsets.UTF_16BE).length == 6;
        boolean euro = "\u20ac".getBytes(StandardCharsets.UTF_8).length == 3;
        return utf8 && utf16 && utf16be && euro ? 1 : 0;
    }

    public static int defaultOffsetWindow() {
        byte[] bytes = "xxhelloyy".getBytes(StandardCharsets.UTF_8);
        return new String(bytes, 2, 5).equals("hello") ? 1 : 0;
    }

    public static int directCharsetOffsetWindow() {
        byte[] bytes = "xxhelloyy".getBytes(StandardCharsets.UTF_8);
        return new String(bytes, 2, 5, StandardCharsets.UTF_8).equals("hello") ? 1 : 0;
    }
}
