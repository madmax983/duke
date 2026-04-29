import java.security.MessageDigest;
import java.security.Provider;
import java.security.SecureRandom;
import java.security.Security;
import java.util.Set;

public class CryptoSpecTest {
    public static int testSha256Digest() throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        byte[] output = digest.digest(new byte[] { 97, 98, 99 });
        if (output.length != 32) {
            return 0;
        }
        return ((output[0] & 255) == 186
                && (output[1] & 255) == 120
                && (output[31] & 255) == 173) ? 1 : 0;
    }

    public static int testMd5DigestCaseInsensitiveLookup() throws Exception {
        MessageDigest digest = MessageDigest.getInstance("md5");
        byte[] output = digest.digest(new byte[] { 97, 98, 99 });
        if (output.length != 16) {
            return 0;
        }
        return ((output[0] & 255) == 144
                && (output[1] & 255) == 1
                && (output[15] & 255) == 114) ? 1 : 0;
    }

    public static int testUpdateThenDigest() throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA1");
        digest.update(new byte[] { 97 });
        digest.update(new byte[] { 98, 99 });
        byte[] output = digest.digest();
        if (output.length != 20) {
            return 0;
        }
        return ((output[0] & 255) == 169
                && (output[1] & 255) == 153
                && (output[19] & 255) == 157) ? 1 : 0;
    }

    public static int testSecureRandomInstantiation() {
        SecureRandom random = new SecureRandom();
        byte[] first = new byte[32];
        byte[] second = new byte[32];
        random.nextBytes(first);
        random.nextBytes(second);
        int diff = 0;
        for (int i = 0; i < first.length; i++) {
            diff |= first[i] ^ second[i];
        }
        return diff != 0 ? 1 : 0;
    }

    public static int testProviderRegistration() throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        Provider provider = digest.getProvider();
        if (provider == null) {
            return 0;
        }
        if (!"DUKE".equals(provider.getName())) {
            return 0;
        }
        if (Security.getProvider("DUKE") == null) {
            return 0;
        }
        Provider[] providers = Security.getProviders();
        if (providers.length < 1) {
            return 0;
        }
        if (provider.getService("MessageDigest", "SHA-256") == null) {
            return 0;
        }
        Set<String> algorithms = Security.getAlgorithms("MessageDigest");
        return algorithms.contains("SHA-256") && algorithms.contains("MD5") ? 1 : 0;
    }
}
