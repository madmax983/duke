import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

public class CharsetClinitInteropTest {
    private static final byte[] HEADER = "duke-magic\n".getBytes(StandardCharsets.UTF_8);
    private static final Charset PROTOCOL_CS = Charset.forName("UTF-8");

    public static int headerLength() {
        return HEADER.length;
    }

    public static int protocolName() {
        return PROTOCOL_CS.name().equals("UTF-8") ? 1 : 0;
    }

    public static void main(String[] args) {
        System.out.println(HEADER.length);
        System.out.println(PROTOCOL_CS.name());
    }
}
