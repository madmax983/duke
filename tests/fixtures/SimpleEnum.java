public class SimpleEnum {
    enum Color { RED, GREEN, BLUE }

    static int testOrdinal() {
        return Color.GREEN.ordinal();  // expect 1
    }

    static int testName() {
        String n = Color.RED.name();
        return n.length();  // "RED" = 3
    }

    static int testValues() {
        Color[] all = Color.values();
        return all.length;  // expect 3
    }

    static int testValueOf() {
        Color c = Color.valueOf("BLUE");
        return c.ordinal();  // expect 2
    }

    static int testSwitch() {
        Color c = Color.GREEN;
        switch (c) {
            case RED: return 10;
            case GREEN: return 20;
            case BLUE: return 30;
            default: return -1;
        }
    }

    static int testEquality() {
        Color a = Color.RED;
        Color b = Color.RED;
        if (a == b) return 1;
        return 0;
    }
}
