import java.util.*;
import java.util.stream.*;

public class Phase41Test {

    // ---- String.format additional specifiers ----

    static int testFormatWidth() {
        // "%5d" pads to width 5
        return String.format("%5d", 42).equals("   42") ? 1 : 0;  // 1
    }

    static int testFormatLeftAlign() {
        return String.format("%-5d", 42).equals("42   ") ? 1 : 0;  // 1
    }

    static int testFormatBooleanSpecifier() {
        return String.format("%b", true).equals("true") ? 1 : 0;  // 1
    }

    static int testFormatCharSpecifier() {
        return String.format("%c", 65).equals("A") ? 1 : 0;  // 1
    }

    static int testFormatOctalSpecifier() {
        return String.format("%o", 8).equals("10") ? 1 : 0;  // 1
    }

    static int testFormatScientific() {
        String s = String.format("%e", 123456.789);
        // Java produces "1.234568e+05" or "1.234568e+005" on some platforms
        return s.contains("1.234568e") ? 1 : 0;  // 1
    }

    static int testFormatZeroPad() {
        return String.format("%05d", 42).equals("00042") ? 1 : 0;  // 1
    }

    static int testFormatPlus() {
        return String.format("%+d", 42).equals("+42") ? 1 : 0;  // 1
    }

    // ---- HashSet.stream() ----

    static int testHashSetStream() {
        HashSet<Integer> set = new HashSet<>();
        set.add(Integer.valueOf(1));
        set.add(Integer.valueOf(2));
        set.add(Integer.valueOf(3));
        return (int) set.stream().count();  // 3
    }

    // ---- LinkedList.stream() ----

    static int testLinkedListStream() {
        LinkedList<String> list = new LinkedList<>();
        list.add("a");
        list.add("b");
        return (int) list.stream().count();  // 2
    }

    // ---- Integer.parseInt(String, int) with radix ----

    static int testParseIntHex() {
        return Integer.parseInt("FF", 16);  // 255
    }

    static int testParseIntBinary() {
        return Integer.parseInt("1010", 2);  // 10
    }

    static int testParseIntOctal() {
        return Integer.parseInt("17", 8);  // 15
    }

    // ---- Long.parseLong(String, int) with radix ----

    static int testParseLongHex() {
        return (int) Long.parseLong("1F", 16);  // 31
    }

    // ---- Integer.toHexString / toBinaryString / toOctalString ----

    static int testIntToHexString() {
        return Integer.toHexString(255).equals("ff") ? 1 : 0;  // 1
    }

    static int testIntToBinaryString() {
        return Integer.toBinaryString(10).equals("1010") ? 1 : 0;  // 1
    }

    static int testIntToOctalString() {
        return Integer.toOctalString(8).equals("10") ? 1 : 0;  // 1
    }

    // ---- Collections.nCopies ----

    static int testCollectionsNCopies() {
        List<String> list = Collections.nCopies(3, "x");
        return list.size();  // 3
    }

    static int testCollectionsNCopiesContent() {
        List<String> list = Collections.nCopies(3, "hi");
        return list.get(0).equals("hi") && list.get(1).equals("hi") ? 1 : 0;  // 1
    }

    // ---- String.chars() returning IntStream ----

    static int testStringChars() {
        return (int) "hello".chars().count();  // 5
    }

    static int testStringCharsSum() {
        // 'a'=97, 'b'=98, 'c'=99 => sum=294
        return "abc".chars().sum();  // 294
    }
}
