import java.nio.charset.StandardCharsets;

public class CharsetMalformedInputTest {
    public static int invalidUtf8ReplacesMalformedBytes() {
        byte[] bytes = new byte[] { (byte)0xc3, 0x28 };
        return new String(bytes, StandardCharsets.UTF_8).equals("\ufffd(") ? 1 : 0;
    }
}
