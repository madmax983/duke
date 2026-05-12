import java.io.IOException;
import java.security.AccessController;
import java.security.PrivilegedAction;
import java.security.PrivilegedActionException;
import java.security.PrivilegedExceptionAction;

@SuppressWarnings("removal")
public class AccessControllerDoPrivilegedTest {
    public static int privilegedActionReturnsString() {
        String value = AccessController.doPrivileged(new PrivilegedAction<String>() {
            public String run() {
                return "duke-access";
            }
        });
        return "duke-access".equals(value) ? 1 : 0;
    }

    public static int privilegedActionReturnsNull() {
        Object value = AccessController.doPrivileged(new PrivilegedAction<Object>() {
            public Object run() {
                return null;
            }
        });
        return value == null ? 1 : 0;
    }

    public static int privilegedActionPropagatesUnchecked() {
        try {
            AccessController.doPrivileged(new PrivilegedAction<Object>() {
                public Object run() {
                    throw new IllegalStateException("duke-unchecked");
                }
            });
            return 0;
        } catch (IllegalStateException expected) {
            return "duke-unchecked".equals(expected.getMessage()) ? 1 : 2;
        }
    }

    public static int privilegedExceptionActionReturnsValue() throws Exception {
        String value = AccessController.doPrivileged(new PrivilegedExceptionAction<String>() {
            public String run() throws Exception {
                return "duke-checked";
            }
        });
        return "duke-checked".equals(value) ? 1 : 0;
    }

    public static int privilegedExceptionActionWrapsCheckedException() {
        try {
            AccessController.doPrivileged(new PrivilegedExceptionAction<Object>() {
                public Object run() throws Exception {
                    throw new IOException("duke-io");
                }
            });
            return 0;
        } catch (PrivilegedActionException expected) {
            Exception cause = expected.getException();
            if (!(cause instanceof IOException)) {
                return 2;
            }
            return "duke-io".equals(cause.getMessage()) ? 1 : 3;
        }
    }

    public static int securityManagerRemainsNullDuringAction() {
        return AccessController.doPrivileged(new PrivilegedAction<Integer>() {
            public Integer run() {
                return System.getSecurityManager() == null ? Integer.valueOf(1) : Integer.valueOf(0);
            }
        }).intValue();
    }
}
