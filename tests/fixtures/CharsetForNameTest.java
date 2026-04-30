import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.nio.charset.UnsupportedCharsetException;

public class CharsetForNameTest {
    public static int utf8Name() {
        return Charset.forName("UTF-8").name().equals("UTF-8") ? 1 : 0;
    }

    public static int utf16Name() {
        return Charset.forName("UTF-16").name().equals("UTF-16") ? 1 : 0;
    }

    public static int utf16beName() {
        return Charset.forName("UTF-16BE").name().equals("UTF-16BE") ? 1 : 0;
    }

    public static int utf16leName() {
        return Charset.forName("UTF-16LE").name().equals("UTF-16LE") ? 1 : 0;
    }

    public static int asciiName() {
        return Charset.forName("US-ASCII").name().equals("US-ASCII") ? 1 : 0;
    }

    public static int latin1Name() {
        return Charset.forName("ISO-8859-1").name().equals("ISO-8859-1") ? 1 : 0;
    }

    public static int aliasesResolveToCanonicalInstances() {
        boolean utf8 = Charset.forName("utf8") == StandardCharsets.UTF_8
            && Charset.forName("UTF8") == StandardCharsets.UTF_8
            && Charset.forName("utf-8") == StandardCharsets.UTF_8;
        boolean latin1 = Charset.forName("latin1") == StandardCharsets.ISO_8859_1;
        boolean ascii = Charset.forName("ASCII") == StandardCharsets.US_ASCII;
        return utf8 && latin1 && ascii ? 1 : 0;
    }

    public static int defaultCharsetIsUtf8() {
        return Charset.defaultCharset().name().equals("UTF-8") ? 1 : 0;
    }

    public static int unsupportedCharsetThrows() {
        try {
            Charset.forName("not-a-charset");
            return 0;
        } catch (UnsupportedCharsetException expected) {
            return 1;
        }
    }

    public static int standardCharsetsUseCanonicalCache() {
        return StandardCharsets.UTF_8 == Charset.forName("UTF-8") ? 1 : 0;
    }

    public static int displayNameAndToStringMatchName() {
        Charset cs = Charset.forName("UTF-16LE");
        return cs.name().equals(cs.displayName()) && cs.name().equals(cs.toString()) ? 1 : 0;
    }

    public static int equalsHashCodeAndRegistered() {
        Charset a = Charset.forName("UTF-8");
        Charset b = Charset.forName("utf8");
        return a.equals(b) && a.hashCode() == b.hashCode() && a.isRegistered() ? 1 : 0;
    }
}
