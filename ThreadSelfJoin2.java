public class ThreadSelfJoin2 {
    public static void main(String[] args) {
        Thread t = new Thread(new Runnable() {
            public void run() {
                try {
                    Thread.currentThread().join();
                } catch (InterruptedException e) {
                    System.out.println("Interrupted");
                }
            }
        });
        t.start();
        try {
            t.join();
        } catch (InterruptedException e) {}
    }
}
