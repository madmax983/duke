import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.StringUtils;

public final class CommonsLang3Smoke {
    private CommonsLang3Smoke() {}

    public static void main(String[] args) {
        String joined = StringUtils.join(new String[] {"duke", "jvm", "smoke"}, '-');
        String capitalized = StringUtils.capitalize("hello");

        int[] numbers = {1, 2, 3};
        int[] extended = ArrayUtils.add(numbers, 4);
        boolean hasFour = ArrayUtils.contains(extended, 4);

        System.out.println("commons-lang3: join=" + joined
            + " capitalize=" + capitalized
            + " added=" + StringUtils.join(ArrayUtils.toObject(extended), ',')
            + " contains4=" + hasFour);
    }
}
