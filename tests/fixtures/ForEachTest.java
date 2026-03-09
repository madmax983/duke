import java.util.ArrayList;

public class ForEachTest {
    public static void main(String[] args) {
        ArrayList<String> list = new ArrayList<>();
        list.add("hello");
        list.add("world");
        for (String s : list) {
            System.out.println(s);
        }
    }
}
