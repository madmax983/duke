public class P08Nestmates {
    private static int secret = 42;
    private int instanceSecret = 7;
    private static String whisper() { return "shh"; }
    private String murmur() { return "mmm"; }

    static class Inner {
        static int readStaticSecret() { return secret; }          // nestmate private static field
        static String callStaticWhisper() { return whisper(); }   // nestmate private static method
        int readInstanceSecret(P08Nestmates o) { return o.instanceSecret; }
        String callInstanceMurmur(P08Nestmates o) { return o.murmur(); }
        Inner(P08Nestmates o) { o.instanceSecret = 99; }          // write nestmate private field
    }

    class InnerNonStatic {
        int grab() { return secret + instanceSecret; }
    }

    public static void main(String[] args) {
        System.out.println("static-field=" + Inner.readStaticSecret());
        System.out.println("static-method=" + Inner.callStaticWhisper());
        P08Nestmates o = new P08Nestmates();
        Inner in = new Inner(o);
        System.out.println("instance-field=" + in.readInstanceSecret(o));
        System.out.println("instance-method=" + in.callInstanceMurmur(o));
        System.out.println("ctor-write=" + o.instanceSecret);
        System.out.println("nonstatic=" + o.new InnerNonStatic().grab());
        // nestmate reflection
        System.out.println("nesthost=" + Inner.class.getNestHost().getSimpleName());
        StringBuilder sb = new StringBuilder();
        for (Class<?> c : P08Nestmates.class.getNestMembers()) sb.append(c.getSimpleName()).append(";");
        System.out.println("nestmembers=" + sb);
        System.out.println("isNestmate=" + Inner.class.isNestmateOf(P08Nestmates.class));
    }
}
