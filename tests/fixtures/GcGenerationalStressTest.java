public class GcGenerationalStressTest {
    public static void main(String[] args) {
        // 10 long-lived objects — must survive all minor GCs.
        String[] longLived = new String[10];
        for (int i = 0; i < 10; i++) {
            longLived[i] = "Survivor-" + i;
        }

        // 5000 short-lived string allocations — trigger multiple minor GCs.
        int sum = 0;
        for (int i = 0; i < 5000; i++) {
            String s = "tmp-" + i;
            sum += s.length();
        }

        // Verify long-lived objects are intact.
        for (int i = 0; i < 10; i++) {
            System.out.println(longLived[i]);
        }

        // Print sum to prevent dead-code elimination.
        System.out.println(sum);
    }
}
