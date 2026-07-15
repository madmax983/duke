/**
 * Regression probe for wave-9 fix (C): {@code Thread.currentThread()} must return a
 * STABLE identity. Two back-to-back calls must yield the SAME object so that
 * reference equality ({@code ==}) holds — the property {@code ReentrantLock}'s
 * exclusive-owner check relies on. Returns 1 when the identities are equal, 0 when
 * a fresh throwaway identity is minted per call (the pre-fix behaviour).
 */
public class ThreadIdentityProbe {
    public static int sameIdentity() {
        Thread a = Thread.currentThread();
        Thread b = Thread.currentThread();
        return (a == b) ? 1 : 0;
    }
}
