import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Base64;

public class Base64Test {
    public static void main(String[] args) throws Exception {
        runAll();
    }

    public static int runAll() throws Exception {
        referenceVectors();
        standardRoundTrips();
        byteArrayOverloads();
        mimeRoundTrip();
        urlRoundTrip();
        invalidInputRaisesIllegalArgumentException();
        cryptoCompose();
        return 1;
    }

    private static void referenceVectors() {
        String[][] vectors = new String[][] {
            new String[] { "", "" },
            new String[] { "f", "Zg==" },
            new String[] { "fo", "Zm8=" },
            new String[] { "foo", "Zm9v" },
            new String[] { "foob", "Zm9vYg==" },
            new String[] { "fooba", "Zm9vYmE=" },
            new String[] { "foobar", "Zm9vYmFy" }
        };

        for (int i = 0; i < vectors.length; i++) {
            byte[] plain = vectors[i][0].getBytes(StandardCharsets.UTF_8);
            String encoded = Base64.getEncoder().encodeToString(plain);
            assertStringEquals(vectors[i][1], encoded);
            assertBytesEquals(plain, Base64.getDecoder().decode(encoded));
        }
    }

    private static void standardRoundTrips() {
        roundTripStandard(new byte[0]);
        roundTripStandard(new byte[] { 1 });
        roundTripStandard(new byte[] { 1, 2 });
        roundTripStandard(new byte[] { 1, 2, 3 });
        roundTripStandard(new byte[] { 1, 2, 3, 4 });
        roundTripStandard(deterministicBytes(255));
    }

    private static void byteArrayOverloads() {
        byte[] plain = new byte[] { 0, 1, 2, 3, 4, 5, 127, (byte) 128, (byte) 255 };
        byte[] encoded = Base64.getEncoder().encode(plain);
        assertStringEquals("AAECAwQFf4D/", new String(encoded, StandardCharsets.UTF_8));
        assertBytesEquals(plain, Base64.getDecoder().decode(encoded));
    }

    private static void mimeRoundTrip() {
        byte[] plain = deterministicBytes(60);
        String encoded = Base64.getMimeEncoder().encodeToString(plain);
        if (encoded.length() != 82 || encoded.charAt(76) != '\r' || encoded.charAt(77) != '\n') {
            throw new AssertionError();
        }
        String withWhitespace = " \t" + encoded.substring(0, 10) + "\r\n"
                + encoded.substring(10) + " \t";
        assertBytesEquals(plain, Base64.getMimeDecoder().decode(withWhitespace));
    }

    private static void urlRoundTrip() {
        byte[] plain = new byte[] { (byte) 251, (byte) 255, (byte) 239, 0, 1, 2 };
        String encoded = Base64.getUrlEncoder().encodeToString(plain);
        assertStringEquals("-__vAAEC", encoded);
        assertBytesEquals(plain, Base64.getUrlDecoder().decode(encoded));
    }

    private static void invalidInputRaisesIllegalArgumentException() {
        try {
            Base64.getDecoder().decode("not!valid!base64!");
        } catch (IllegalArgumentException expected) {
            return;
        }
        throw new AssertionError();
    }

    private static void cryptoCompose() throws Exception {
        byte[] digest = MessageDigest.getInstance("SHA-256")
                .digest("hello".getBytes(StandardCharsets.UTF_8));
        String encoded = Base64.getEncoder().encodeToString(digest);
        assertStringEquals("LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=", encoded);
    }

    private static void roundTripStandard(byte[] plain) {
        String encoded = Base64.getEncoder().encodeToString(plain);
        assertBytesEquals(plain, Base64.getDecoder().decode(encoded));
    }

    private static byte[] deterministicBytes(int length) {
        byte[] bytes = new byte[length];
        for (int i = 0; i < bytes.length; i++) {
            bytes[i] = (byte) (i * 31 + 7);
        }
        return bytes;
    }

    private static void assertStringEquals(String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError();
        }
    }

    private static void assertBytesEquals(byte[] expected, byte[] actual) {
        if (expected.length != actual.length) {
            throw new AssertionError();
        }
        for (int i = 0; i < expected.length; i++) {
            if (expected[i] != actual[i]) {
                throw new AssertionError();
            }
        }
    }
}
