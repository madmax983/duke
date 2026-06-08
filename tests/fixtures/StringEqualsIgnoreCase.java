// Fixture for issue #854: java.lang.String.equalsIgnoreCase native.
// A single `main` exercises every Acceptance Criterion bullet and prints
// "OK" iff all of them hold; otherwise it prints "FAIL".
public class StringEqualsIgnoreCase {
    public static void main(String[] args) {
        boolean ok = true;

        // AC: Null arg -> false (argument is not null is part of the contract).
        ok &= !"INFO".equalsIgnoreCase(null);

        // AC: Length mismatch -> false without scanning characters.
        ok &= !"abc".equalsIgnoreCase("ab");

        // AC: ASCII case fold.
        ok &= "INFO".equalsIgnoreCase("info");
        ok &= "Info".equalsIgnoreCase("iNfO");
        ok &= !"info".equalsIgnoreCase("warn");

        // AC: Locale-independent two-step fold. 'k' (U+006B) and the KELVIN
        // SIGN (U+212A) are unequal under a naive ASCII fold, but Java treats
        // them as equal because Character.toLowerCase(U+212A) == 'k'.
        ok &= "k".equalsIgnoreCase("K");

        // AC: Reflexive on identical content (and receiver-equals-self).
        ok &= "hello".equalsIgnoreCase("hello");
        String self = "hello";
        ok &= self.equalsIgnoreCase(self);

        System.out.println(ok ? "OK" : "FAIL");
    }
}
