import java.util.EnumSet;
import java.util.Iterator;

public class EnumSetTest {
    enum Day { MON, TUE, WED, THU, FRI, SAT, SUN }

    static int testOfContains() {
        return EnumSet.of(Day.MON, Day.WED).contains(Day.WED) ? 1 : 0;  // expect 1
    }

    static int testOfNotContains() {
        return EnumSet.of(Day.MON, Day.WED).contains(Day.TUE) ? 1 : 0;  // expect 0
    }

    static int testNoneOfSize() {
        return EnumSet.noneOf(Day.class).size();  // expect 0
    }

    static int testAllOfSize() {
        return EnumSet.allOf(Day.class).size();  // expect 7
    }

    static int testRangeSize() {
        return EnumSet.range(Day.TUE, Day.THU).size();  // expect 3
    }

    static int testAddRemove() {
        EnumSet<Day> s = EnumSet.noneOf(Day.class);
        int r = 0;
        if (s.add(Day.MON)) r += 1;    // first add returns true
        if (!s.add(Day.MON)) r += 2;   // second add returns false
        s.remove(Day.MON);
        r += s.size() * 10;            // size 0 after removal
        return r;                      // expect 3
    }

    static int testIteratorCount() {
        EnumSet<Day> s = EnumSet.of(Day.MON, Day.FRI, Day.SUN);
        int c = 0;
        Iterator<Day> it = s.iterator();
        while (it.hasNext()) { it.next(); c++; }
        return c;  // expect 3
    }

    static int testIsEnum() {
        return Day.class.isEnum() ? 1 : 0;  // expect 1
    }

    static int testStringNotEnum() {
        return String.class.isEnum() ? 1 : 0;  // expect 0
    }

    static int testGetEnumConstantsLen() {
        return Day.class.getEnumConstants().length;  // expect 7
    }
}
