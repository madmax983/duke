public class P05TextBlocks {
    public static void main(String[] args) {
        String html = """
                <html>
                    <body>hi</body>
                </html>
                """;
        System.out.println("textblock-lines=" + html.lines().count());
        System.out.println("textblock-first=" + html.lines().findFirst().orElse("?"));

        String json = """
                {
                    "a": 1,
                    "b": "x"
                }
                """;
        System.out.println("json-indent-ok=" + json.startsWith("{"));

        String q = """
                select * from t where a = 'x' and b = "y\"""";
        System.out.println("quotes=" + q.contains("\"y\""));

        // string concat via invokedynamic
        String name = "duke";
        int n = 21;
        String s = "hello " + name + "! java=" + n + " pi=" + 3.14 + " b=" + true;
        System.out.println("concat=" + s);
        System.out.println("concat2=" + ("a" + 1 + "b" + 2L + "c" + 'd'));

        // newer String APIs
        System.out.println("repeat=" + "ab".repeat(3));
        System.out.println("strip=" + "  hi  ".strip());
        System.out.println("isblank=" + "   ".isBlank() + "," + "".isBlank() + "," + "x".isBlank());
        System.out.println("lines-count=" + "a\nb\nc".lines().count());
        System.out.println("indent=" + "x".indent(2).length());
        System.out.println("transform=" + "hi".transform(String::toUpperCase));
        System.out.println("formatted=" + "%s=%d".formatted("n", 42));
        String esc = "a\\tb".translateEscapes();
        System.out.println("translate-escapes=" + esc.length() + "," + (esc.charAt(1) == '\t'));
        System.out.println("strip-indent=" + "    a\n    b\n".stripIndent().equals("a\nb\n"));
    }
}
