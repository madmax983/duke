import java.util.LinkedList;

public class LinkedListTest {
    static int testSize() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.add(1);
        ll.add(2);
        ll.add(3);
        return ll.size();  // 3
    }

    static int testPeekFirst() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.add(10);
        ll.add(20);
        return (Integer) ll.peekFirst();  // 10
    }

    static int testRemoveFirst() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.add(5);
        ll.add(6);
        ll.removeFirst();
        return ll.size();  // 1
    }

    static int testAddFirst() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.addFirst(2);
        ll.addFirst(1);
        return (Integer) ll.removeFirst();  // 1
    }

    static int testPeekLast() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.add(7);
        ll.add(8);
        ll.add(9);
        return (Integer) ll.peekLast();  // 9
    }

    static int testPoll() {
        LinkedList<Integer> ll = new LinkedList<>();
        ll.add(100);
        ll.add(200);
        int first = (Integer) ll.poll();
        return ll.size() + first;  // 1 + 100 = 101
    }
}
