import java.io.UnsupportedEncodingException;

public class StringBytesNamedCharsetTest {
    public static int getBytesUtf8ByName() throws Exception {
        return "\u20ac".getBytes("UTF-8").length == 3 ? 1 : 0;
    }

    public static int newStringIsoByName() throws Exception {
        byte[] bytes = new byte[] { (byte)0x43, (byte)0x61, (byte)0x66, (byte)0xe9 };
        return new String(bytes, "ISO-8859-1").equals("Caf\u00e9") ? 1 : 0;
    }

    public static int defaultBytesUseUtf8() {
        byte[] bytes = "\u20ac".getBytes();
        return bytes.length == 3 && new String(bytes).equals("\u20ac") ? 1 : 0;
    }

    public static int namedOffsetWindow() throws Exception {
        byte[] bytes = "xxhelloyy".getBytes("UTF-8");
        return new String(bytes, 2, 5, "UTF-8").equals("hello") ? 1 : 0;
    }

    public static int getBytesUnknownThrowsUnsupportedEncoding() {
        try {
            "x".getBytes("not-a-charset");
            return 0;
        } catch (UnsupportedEncodingException expected) {
            return 1;
        }
    }

    public static int constructorUnknownThrowsUnsupportedEncoding() {
        try {
            new String(new byte[] { 1 }, "not-a-charset");
            return 0;
        } catch (UnsupportedEncodingException expected) {
            return 1;
        }
    }

    public static int aliasesWorkForNamedStringOverloads() throws Exception {
        boolean utf8 = new String("ok".getBytes("utf8"), "UTF8").equals("ok");
        boolean latin1 = new String(new byte[] { (byte)0xe9 }, "latin1").equals("\u00e9");
        return utf8 && latin1 ? 1 : 0;
    }
}
