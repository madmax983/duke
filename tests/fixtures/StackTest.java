import java.util.Stack;

public class StackTest {
    static int testPushPop() {
        Stack<Integer> stack = new Stack<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        return (Integer) stack.pop();  // 3
    }

    static int testPeek() {
        Stack<Integer> stack = new Stack<>();
        stack.push(10);
        stack.push(20);
        return (Integer) stack.peek();  // 20
    }

    static int testSize() {
        Stack<Integer> stack = new Stack<>();
        stack.push(1);
        stack.push(2);
        return stack.size();  // 2
    }

    static int testEmpty() {
        Stack<Integer> stack = new Stack<>();
        return stack.empty() ? 1 : 0;  // 1
    }

    static int testNotEmpty() {
        Stack<Integer> stack = new Stack<>();
        stack.push(5);
        return stack.empty() ? 0 : 1;  // 1
    }
}
