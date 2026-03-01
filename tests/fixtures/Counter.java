/** A slightly more complex class: static field, constructor, instance method. */
public class Counter {
    private static int instanceCount = 0;
    private int value;

    public Counter(int initial) {
        this.value = initial;
        instanceCount++;
    }

    public int increment() {
        return ++value;
    }

    public static int getInstanceCount() {
        return instanceCount;
    }
}
