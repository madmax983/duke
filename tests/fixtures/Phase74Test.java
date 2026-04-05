import java.util.*;
import java.util.stream.*;

public class Phase74Test {

    // TreeMap operations
    public static int testTreeMapOrdered() {
        TreeMap<Integer, String> map = new TreeMap<>();
        map.put(3, "three"); map.put(1, "one"); map.put(2, "two");
        // firstKey and lastKey
        return map.firstKey() * 10 + map.lastKey(); // 1*10 + 3 = 13
    }

    // TreeMap.headMap / tailMap
    public static int testTreeMapHeadTail() {
        TreeMap<Integer, Integer> map = new TreeMap<>();
        for (int i = 1; i <= 5; i++) map.put(i, i * 10);
        int headSize = map.headMap(3).size(); // keys < 3: {1,2} = 2
        int tailSize = map.tailMap(3).size(); // keys >= 3: {3,4,5} = 3
        return headSize * 10 + tailSize; // 23
    }

    // Stack operations
    public static int testStack() {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1); stack.push(2); stack.push(3);
        int top = stack.pop(); // 3
        int next = stack.peek(); // 2
        return top * 10 + next; // 32
    }

    // Queue operations (ArrayDeque as queue)
    public static int testQueue() {
        Queue<Integer> q = new LinkedList<>();
        q.offer(1); q.offer(2); q.offer(3);
        int head = q.poll(); // 1
        int peek = q.peek(); // 2
        return head * 10 + peek; // 12
    }

    // PriorityQueue
    public static int testPriorityQueue() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(5); pq.offer(1); pq.offer(3);
        int first = pq.poll(); // 1 (min-heap)
        int second = pq.poll(); // 3
        return first * 10 + second; // 13
    }

    // Map.entry iteration
    public static int testMapEntrySet() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        int sum = 0;
        for (Map.Entry<String, Integer> e : m.entrySet()) {
            sum += e.getValue();
        }
        return sum; // 6
    }

    // Collections.frequency
    public static int testCollectionsFrequency() {
        List<String> list = new ArrayList<>();
        list.add("a"); list.add("b"); list.add("a"); list.add("c"); list.add("a");
        return Collections.frequency(list, "a"); // 3
    }

    // Collections.min / max
    public static int testCollectionsMinMax() {
        List<Integer> list = new ArrayList<>();
        list.add(5); list.add(2); list.add(8); list.add(1); list.add(6);
        int min = Collections.min(list);
        int max = Collections.max(list);
        return min * 10 + max; // 1*10 + 8 = 18
    }
}
