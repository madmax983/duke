import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.*;

public class P06VirtualThreads {
    public static void main(String[] args) throws Exception {
        // basic start/join
        List<String> out = Collections.synchronizedList(new ArrayList<>());
        Thread vt = Thread.ofVirtual().name("worker-1").start(() -> {
            out.add("ran:" + Thread.currentThread().isVirtual());
            out.add("name:" + Thread.currentThread().getName());
        });
        System.out.println("isVirtual=" + vt.isVirtual());
        System.out.println("isAlive-before-join=" + vt.isAlive());
        vt.join();
        System.out.println("isAlive-after-join=" + vt.isAlive());
        Collections.sort(out);
        out.forEach(System.out::println);

        // unstarted virtual thread
        Thread u = Thread.ofVirtual().unstarted(() -> {});
        System.out.println("unstarted-virtual=" + u.isVirtual() + "," + u.isAlive());
        u.start(); u.join();
        System.out.println("unstarted-ran=" + !u.isAlive());

        // platform thread still works
        Thread pt = Thread.ofPlatform().name("plat").start(() -> {});
        System.out.println("platform-virtual=" + pt.isVirtual());
        pt.join();

        // executor: one virtual thread per task
        List<Integer> results = Collections.synchronizedList(new ArrayList<>());
        try (var exec = Executors.newVirtualThreadPerTaskExecutor()) {
            for (int i = 0; i < 10; i++) {
                final int k = i;
                exec.submit(() -> { results.add(k * k); return null; });
            }
        }
        Collections.sort(results);
        System.out.println("executor=" + results);

        // ThreadLocal isolation across virtual threads
        ThreadLocal<String> tl = new ThreadLocal<>();
        tl.set("main-value");
        List<String> seen = Collections.synchronizedList(new ArrayList<>());
        Thread t1 = Thread.ofVirtual().start(() -> { tl.set("t1"); seen.add(tl.get()); });
        Thread t2 = Thread.ofVirtual().start(() -> seen.add(String.valueOf(tl.get())));
        t1.join(); t2.join();
        Collections.sort(seen);
        System.out.println("threadlocal=" + seen + ",main=" + tl.get());

        // exception propagation via Future
        try (var exec = Executors.newVirtualThreadPerTaskExecutor()) {
            Future<?> f = exec.submit(() -> { throw new RuntimeException("boom"); });
            try {
                f.get();
                System.out.println("future-ex=no-throw");
            } catch (ExecutionException ee) {
                System.out.println("future-ex=" + ee.getCause().getMessage());
            }
        }

        // join many
        List<Thread> threads = new ArrayList<>();
        for (int i = 0; i < 50; i++) threads.add(Thread.ofVirtual().start(() -> {}));
        for (Thread t : threads) t.join();
        System.out.println("join50=done");

        // virtual thread factory naming
        ThreadFactory f = Thread.ofVirtual().name("vt-", 0).factory();
        Thread named = f.newThread(() -> {});
        System.out.println("factory-name=" + named.getName() + "," + named.isVirtual());
    }
}
