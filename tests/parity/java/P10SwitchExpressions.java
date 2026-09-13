public class P10SwitchExpressions {
    enum Day { MON, TUE, WED, THU, FRI, SAT, SUN }

    static String kind(Day d) {
        return switch (d) {
            case MON, TUE, WED, THU, FRI -> "weekday";
            case SAT, SUN -> "weekend";
        };
    }

    static int withYield(int x) {
        return switch (x) {
            case 0: yield 100;
            case 1, 2: yield 200;
            default: {
                int t = x * 2;
                yield t + 1;
            }
        };
    }

    public static void main(String[] args) {
        System.out.println(kind(Day.WED) + "," + kind(Day.SUN));
        System.out.println(withYield(0) + "," + withYield(2) + "," + withYield(5));

        // switch expression over String with arrows
        String s = "b";
        int v = switch (s) {
            case "a" -> 1;
            case "b" -> 2;
            default -> 0;
        };
        System.out.println("str-switch=" + v);

        // var in switch
        var day = Day.MON;
        var label = switch (day) {
            case MON -> "monday";
            default -> "other";
        };
        System.out.println("var=" + label);

        // nested switch expressions
        int code = 21;
        String r = switch (code / 10) {
            case 2 -> switch (code % 10) {
                case 1 -> "twenty-one";
                default -> "twenty-x";
            };
            default -> "other";
        };
        System.out.println("nested=" + r);
    }
}
