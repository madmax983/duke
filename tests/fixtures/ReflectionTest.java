import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;

public final class ReflectionTest {
    public static int forNameAndGetName() throws Exception {
        Class<?> cls = Class.forName("ReflectionTarget");
        return cls.getName().equals("ReflectionTarget") ? 1 : 0;
    }

    public static int stringClassLiteralUsesBinaryName() {
        return String.class.getName().equals("java.lang.String") ? 1 : 0;
    }

    public static int declaredMethodsIncludePublicAndPrivate() {
        int sawAdd = 0;
        int sawTimes = 0;
        int sawHidden = 0;
        int sawExplode = 0;
        int sawAddLong = 0;

        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            String name = method.getName();
            if (name.equals("add")) sawAdd = 1;
            if (name.equals("times")) sawTimes = 1;
            if (name.equals("hidden")) sawHidden = 1;
            if (name.equals("explode")) sawExplode = 1;
            if (name.equals("addLong")) sawAddLong = 1;
        }

        return sawAdd + sawTimes + sawHidden + sawExplode + sawAddLong;
    }

    public static int declaredFieldsIncludePublicAndPrivate() {
        int sawBase = 0;
        int sawSecret = 0;

        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            String name = field.getName();
            if (name.equals("base")) sawBase = 1;
            if (name.equals("secret")) sawSecret = 1;
        }

        return sawBase + sawSecret;
    }

    public static int invokeStaticAdd() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("add")) {
                Object result = method.invoke(null, Integer.valueOf(2), Integer.valueOf(5));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeInstanceTimes() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("times")) {
                Object result = method.invoke(target, Integer.valueOf(3));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeStaticAddLong() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("addLong")) {
                Object result = method.invoke(null, Long.valueOf(2L), Long.valueOf(5L));
                return ((Long) result).longValue() == 7L ? 1 : 0;
            }
        }
        return -1;
    }

    public static int missingClassRaisesClassNotFound() {
        try {
            Class.forName("duke.missing.ReflectionGhost");
            return 0;
        } catch (ClassNotFoundException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int privateMethodRaisesIllegalAccess() throws Exception {
        ReflectionTarget target = new ReflectionTarget(3, 13);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("hidden")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (IllegalAccessException e) {
                    return 1;
                }
            }
        }
        return -1;
    }

    public static int targetExceptionIsWrapped() throws Exception {
        ReflectionTarget target = new ReflectionTarget(1, 2);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("explode")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (InvocationTargetException e) {
                    return 1;
                }
            }
        }
        return -1;
    }
}

final class ReflectionTarget {
    public int base;
    private int secret;

    ReflectionTarget(int base, int secret) {
        this.base = base;
        this.secret = secret;
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static long addLong(long a, long b) {
        return a + b;
    }

    public int times(int factor) {
        return base * factor;
    }

    private int hidden() {
        return secret;
    }

    public void explode() {
        throw new RuntimeException("boom");
    }
}
