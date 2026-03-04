public class ClassLiteral {
    // Returns 1 if String.class is non-null
    static int testStringClass() {
        Class<?> c = String.class;
        if (c != null) return 1;
        return 0;
    }

    // Returns 1 if ClassLiteral.class is non-null
    static int testPrimitiveClass() {
        Class<?> c = ClassLiteral.class;
        if (c != null) return 1;
        return 0;
    }

    // Two ldc of same class should give same reference (interned)
    static int testClassInterning() {
        Class<?> a = String.class;
        Class<?> b = String.class;
        if (a == b) return 1;
        return 0;
    }
}
