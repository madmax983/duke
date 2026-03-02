/** Tests exception hierarchy matching in catch clauses. */
public class ExceptionHierarchy {
    /** Throws RuntimeException, catches with Exception — should succeed. */
    public static int catchParent() {
        try {
            throw new RuntimeException("test");
        } catch (Exception e) {
            return 1;
        }
    }

    /** Throws RuntimeException, catches with RuntimeException — exact match. */
    public static int catchExact() {
        try {
            throw new RuntimeException("test");
        } catch (RuntimeException e) {
            return 2;
        }
    }

    /** Throws Exception, tries RuntimeException first (should NOT match), then Exception. */
    public static int catchWrongThenRight() {
        try {
            throw new Exception("test");
        } catch (RuntimeException e) {
            return 0;
        } catch (Exception e) {
            return 3;
        }
    }

    /** Catches with Throwable (grandparent of RuntimeException). */
    public static int catchGrandparent() {
        try {
            throw new RuntimeException("test");
        } catch (Throwable t) {
            return 4;
        }
    }
}
