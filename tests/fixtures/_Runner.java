import java.lang.reflect.*;
public class _Runner {
    public static void main(String[] args) throws Exception {
        Class<?> c = Class.forName(args[0]);
        Method m = c.getMethod(args[1]);
        System.out.println(m.invoke(null));
    }
}
