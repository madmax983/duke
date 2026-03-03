public class StringConcatTest {
    public static int testSimple() {
        String name = "World";
        String result = "Hello, " + name + "!";
        return result.equals("Hello, World!") ? 1 : 0;
    }

    public static int testInt() {
        int x = 42;
        String result = "Value: " + x;
        return result.equals("Value: 42") ? 1 : 0;
    }

    public static int testChain() {
        int a = 3, b = 5;
        String result = a + " + " + b + " = " + (a + b);
        return result.equals("3 + 5 = 8") ? 1 : 0;
    }

    public static int testBoolean() {
        boolean flag = true;
        String result = "active=" + flag;
        return result.equals("active=true") ? 1 : 0;
    }

    public static int testEmpty() {
        String s = "";
        String result = s + "ok";
        return result.equals("ok") ? 1 : 0;
    }
}
