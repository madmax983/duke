public class MainHello {
    public static void main(String[] args) {
        if (args.length == 0) {
            System.out.println("no args");
        } else {
            for (int i = 0; i < args.length; i++) {
                System.out.println(args[i]);
            }
        }
    }

    /** Returns number of args. For testing without main(). */
    public static int countArgs(String[] args) {
        return args.length;
    }
}
