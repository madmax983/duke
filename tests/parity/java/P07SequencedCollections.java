import java.util.*;

public class P07SequencedCollections {
    public static void main(String[] args) {
        // LinkedHashMap as SequencedMap
        SequencedMap<String, Integer> sm = new LinkedHashMap<>();
        sm.put("a", 1); sm.put("b", 2); sm.put("c", 3);
        System.out.println("first=" + sm.firstEntry() + ",last=" + sm.lastEntry());
        sm.putFirst("z", 0);
        sm.putLast("y", 9);
        System.out.println("order=" + sm.keySet());
        System.out.println("pollFirst=" + sm.pollFirstEntry() + ",pollLast=" + sm.pollLastEntry());
        System.out.println("reversed=" + sm.reversed().keySet());

        // LinkedHashSet as SequencedSet
        SequencedSet<String> ss = new LinkedHashSet<>(List.of("x", "y"));
        ss.addFirst("a"); ss.addLast("z");
        System.out.println("set=" + ss + ",first=" + ss.getFirst() + ",last=" + ss.getLast());
        System.out.println("set-reversed=" + ss.reversed());
        System.out.println("removeFirst=" + ss.removeFirst() + ",left=" + ss);

        // TreeMap sequenced view
        SequencedMap<Integer, String> tm = new TreeMap<>();
        tm.put(2, "b"); tm.put(1, "a"); tm.put(3, "c");
        System.out.println("treemap=" + tm.keySet() + "," + tm.firstEntry().getKey());

        // List.reversed (Java 21)
        List<Integer> list = new ArrayList<>(List.of(1, 2, 3));
        System.out.println("list-reversed=" + list.reversed());
        list.addFirst(0); list.addLast(4);
        System.out.println("list=" + list + "," + list.getFirst() + "," + list.getLast());
        System.out.println("list-remove=" + list.removeFirst() + "," + list.removeLast() + "," + list);

        // Deque still fine
        Deque<String> dq = new ArrayDeque<>();
        dq.addFirst("f"); dq.addLast("l");
        System.out.println("deque=" + dq.getFirst() + "," + dq.getLast());

        // SequencedCollection interface default methods
        SequencedCollection<String> sc = new ArrayList<>(List.of("m", "n"));
        System.out.println("seqcoll=" + sc.getFirst() + "," + sc.getLast());
    }
}
