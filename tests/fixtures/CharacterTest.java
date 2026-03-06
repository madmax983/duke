public class CharacterTest {
    static int testIsDigit() {
        if (Character.isDigit('5')) return 1;
        return 0;
    }

    static int testIsDigitFalse() {
        if (Character.isDigit('A')) return 0;
        return 1;
    }

    static int testIsLetter() {
        if (Character.isLetter('Z')) return 1;
        return 0;
    }

    static int testIsLetterFalse() {
        if (Character.isLetter('3')) return 0;
        return 1;
    }

    static int testIsWhitespace() {
        if (Character.isWhitespace(' ')) return 1;
        return 0;
    }

    static int testIsUpperCase() {
        if (Character.isUpperCase('A') && !Character.isUpperCase('a')) return 1;
        return 0;
    }

    static int testIsLowerCase() {
        if (Character.isLowerCase('z') && !Character.isLowerCase('Z')) return 1;
        return 0;
    }

    static int testToUpperCase() {
        char c = Character.toUpperCase('a');
        return (int) c;  // 'A' = 65
    }

    static int testToLowerCase() {
        char c = Character.toLowerCase('A');
        return (int) c;  // 'a' = 97
    }

    static int testIsLetterOrDigit() {
        if (Character.isLetterOrDigit('a') && Character.isLetterOrDigit('5')
            && !Character.isLetterOrDigit(' ')) return 1;
        return 0;
    }

    static int testValueOf() {
        Character c = Character.valueOf('X');
        char v = c.charValue();
        return (int) v;  // 'X' = 88
    }
}
