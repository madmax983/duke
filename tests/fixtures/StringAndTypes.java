public class StringAndTypes {

    /**
     * Load a string constant via ldc.
     * Returns 1 if the string reference is non-null, 0 otherwise.
     * Exercises: ldc for CpEntry::String, ifnonnull
     */
    public static int stringNonNull() {
        String s = "hello";
        if (s != null) {
            return 1;
        }
        return 0;
    }

    /**
     * Two ldc of the same string literal should return the same reference
     * (string interning). Returns 1 if same ref, 0 otherwise.
     * Exercises: ldc String, if_acmpeq
     */
    public static int stringIntern() {
        String a = "hello";
        String b = "hello";
        if (a == b) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with exact class match.
     * Creates a RuntimeException, checks instanceof RuntimeException → true.
     * Exercises: new, instanceof
     */
    public static int instanceOfMatch() {
        Object o = new RuntimeException();
        if (o instanceof RuntimeException) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with non-matching class.
     * Creates a RuntimeException, checks instanceof Error → false.
     * Exercises: instanceof with class mismatch
     */
    public static int instanceOfMismatch() {
        Object o = new RuntimeException();
        if (o instanceof Error) {
            return 1;
        }
        return 0;
    }

    /**
     * instanceof with null → always false.
     * Exercises: instanceof with null reference
     */
    public static int instanceOfNull() {
        Object o = null;
        if (o instanceof RuntimeException) {
            return 1;
        }
        return 0;
    }

    /**
     * checkcast that succeeds (exact match).
     * Exercises: checkcast with matching type
     */
    public static int checkcastOk() {
        Object o = new RuntimeException();
        RuntimeException e = (RuntimeException) o; // checkcast
        return 42;
    }

    /**
     * if_acmpeq: same reference comparison.
     * Exercises: if_acmpeq
     */
    public static int refEqual() {
        int[] a = new int[1];
        int[] b = a;
        if (a == b) {
            return 1;
        }
        return 0;
    }

    /**
     * if_acmpne: different reference comparison.
     * Exercises: if_acmpne
     */
    public static int refNotEqual() {
        int[] a = new int[1];
        int[] b = new int[1];
        if (a != b) {
            return 1;
        }
        return 0;
    }
}
