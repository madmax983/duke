public class StringBuilderExtTest {
    static int testInsertString() {
        StringBuilder sb = new StringBuilder("hello");
        sb.insert(2, "XY");
        return sb.length();  // 7
    }

    static int testInsertStringValue() {
        StringBuilder sb = new StringBuilder("ace");
        sb.insert(1, "b");
        return sb.toString().equals("abce") ? 1 : 0;  // 1
    }

    static int testDelete() {
        StringBuilder sb = new StringBuilder("hello");
        sb.delete(1, 3);
        return sb.length();  // 3
    }

    static int testDeleteCharAt() {
        StringBuilder sb = new StringBuilder("hello");
        sb.deleteCharAt(2);
        return sb.length();  // 4
    }

    static int testReverse() {
        StringBuilder sb = new StringBuilder("abc");
        sb.reverse();
        return sb.toString().equals("cba") ? 1 : 0;  // 1
    }

    static int testCharAt() {
        StringBuilder sb = new StringBuilder("hello");
        return sb.charAt(1);  // 'e' = 101
    }

    static int testSetLength() {
        StringBuilder sb = new StringBuilder("hello");
        sb.setLength(3);
        return sb.length();  // 3
    }
}
