public class StringOps2 {
    public static int testToUpperCase() {
        String s = "hello world";
        String u = s.toUpperCase();
        return u.equals("HELLO WORLD") ? 1 : 0;
    }
    public static int testToLowerCase() {
        String s = "Hello World";
        String l = s.toLowerCase();
        return l.equals("hello world") ? 1 : 0;
    }
    public static int testReplace() {
        String s = "hello world";
        String r = s.replace('l', 'r');
        return r.equals("herro worrd") ? 1 : 0;
    }
    public static int testReplaceString() {
        String s = "hello world hello";
        String r = s.replace("hello", "hi");
        return r.equals("hi world hi") ? 1 : 0;
    }
    public static int testSplit() {
        String s = "a,b,c,d";
        String[] parts = s.split(",");
        return (parts.length == 4 && parts[0].equals("a") && parts[3].equals("d")) ? 1 : 0;
    }
    public static int testHashCode() {
        String s1 = "hello";
        String s2 = "hello";
        return (s1.hashCode() == s2.hashCode()) ? 1 : 0;
    }
    public static int testToStringIdentity() {
        String s = "test";
        String t = s.toString();
        return s.equals(t) ? 1 : 0;
    }
    public static int testReplaceCharSequence() {
        String s = "foo bar baz";
        String r = s.replace("bar", "qux");
        return r.equals("foo qux baz") ? 1 : 0;
    }
}
