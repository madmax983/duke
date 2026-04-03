import java.util.PriorityQueue;

public class PriorityQueueTest {
    static int testPollOrder() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(3);
        pq.offer(1);
        pq.offer(2);
        return pq.poll();  // 1 (min-heap)
    }

    static int testPeekMin() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(5);
        pq.offer(2);
        pq.offer(8);
        return pq.peek();  // 2
    }

    static int testSize() {
        PriorityQueue<String> pq = new PriorityQueue<>();
        pq.offer("c");
        pq.offer("a");
        pq.offer("b");
        return pq.size();  // 3
    }

    static int testIsEmpty() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        int before = pq.isEmpty() ? 1 : 0;
        pq.offer(1);
        int after = pq.isEmpty() ? 1 : 0;
        return before + after;  // 1
    }

    static int testPollAll() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(5);
        pq.offer(1);
        pq.offer(3);
        int sum = 0;
        while (!pq.isEmpty()) {
            sum += pq.poll();
        }
        return sum;  // 9
    }
}
