import java.util.ArrayDeque;
import java.util.Deque;

public class ArrayDequeTest {
    static int testPushPop() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.push(1);
        deque.push(2);
        deque.push(3);
        return deque.pop();  // 3 (stack behavior: LIFO)
    }

    static int testOfferPoll() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.offer(10);
        deque.offer(20);
        deque.offer(30);
        return deque.poll();  // 10 (queue behavior: FIFO)
    }

    static int testPeek() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.offer(5);
        deque.offer(6);
        return deque.peek();  // 5
    }

    static int testSize() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.add("a");
        deque.add("b");
        return deque.size();  // 2
    }

    static int testIsEmpty() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        int before = deque.isEmpty() ? 1 : 0;
        deque.add(1);
        int after = deque.isEmpty() ? 1 : 0;
        return before + after;  // 1 (was empty, now not)
    }
}
