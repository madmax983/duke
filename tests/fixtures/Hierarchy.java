/** Tests class hierarchy for instanceof and checkcast. */
public class Hierarchy {
    /** Returns 1 if a Hierarchy instance passes instanceof Object. */
    public static int instanceOfObject() {
        Object obj = new Hierarchy();
        return (obj instanceof Object) ? 1 : 0;
    }

    /** Returns 1 if casting Hierarchy to Object works. */
    public static int castToObject() {
        Hierarchy h = new Hierarchy();
        Object obj = (Object) h;
        return (obj != null) ? 1 : 0;
    }

    /** Returns 0 since null instanceof anything is false. */
    public static int nullInstanceOf() {
        Object obj = null;
        return (obj instanceof Hierarchy) ? 1 : 0;
    }
}
