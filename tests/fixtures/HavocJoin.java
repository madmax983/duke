public final class HavocJoin {
    static final class Worker extends Thread {
        @Override
        public void run() {
            try {
                Thread.sleep(200);
            } catch (Exception e) {}
        }
    }
    static final class Joiner extends Thread {
        private final Thread target;
        Joiner(Thread target) {
            this.target = target;
        }
        @Override
        public void run() {
            try {
                target.join();
            } catch (Exception e) {}
        }
    }
    public static int main() throws Exception {
        Worker w = new Worker();
        Joiner j1 = new Joiner(w);
        Joiner j2 = new Joiner(w);
        w.start();
        j1.start();
        j2.start();
        j1.join();
        j2.join();
        w.join();
        return 0;
    }
}
