import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.reflect.Field;
import java.lang.reflect.Method;

enum DukeMode {
    ALPHA,
    BETA
}

@Retention(RetentionPolicy.RUNTIME)
@interface DukeNested {
    int n() default 9;
}

@Retention(RetentionPolicy.RUNTIME)
@interface DukeAnno {
    String name() default "default-name";

    int count();

    Class<?> target() default String.class;

    DukeMode mode() default DukeMode.ALPHA;

    int[] weights() default {2, 3};

    DukeNested nested() default @DukeNested(n = 9);
}

@DukeAnno(
        name = "classy",
        count = 42,
        mode = DukeMode.BETA,
        weights = {4, 5},
        nested = @DukeNested(n = 11))
public final class AnnotationTest {
    @DukeAnno(name = "fieldy", count = 5)
    public int annotatedField;

    @DukeAnno(name = "methody", count = 7)
    public static int annotatedMethod() {
        return 1;
    }

    public static int classAnnotationConfiguredAndDefaults() {
        DukeAnno ann = AnnotationTest.class.getAnnotation(DukeAnno.class);
        if (ann == null) return 0;
        return ann.name().equals("classy")
                        && ann.count() == 42
                        && ann.target() == String.class
                        && ann.mode() == DukeMode.BETA
                        && ann.weights().length == 2
                        && ann.weights()[1] == 5
                        && ann.nested().n() == 11
                ? 1
                : 0;
    }

    public static int methodAndFieldAnnotations() throws Exception {
        Method method = AnnotationTest.class.getDeclaredMethod("annotatedMethod");
        Field field = AnnotationTest.class.getDeclaredField("annotatedField");
        DukeAnno methodAnn = method.getAnnotation(DukeAnno.class);
        DukeAnno fieldAnn = field.getAnnotation(DukeAnno.class);
        if (methodAnn == null || fieldAnn == null) return 0;
        return methodAnn.name().equals("methody")
                        && methodAnn.count() == 7
                        && methodAnn.target() == String.class
                        && fieldAnn.name().equals("fieldy")
                        && fieldAnn.count() == 5
                        && fieldAnn.mode() == DukeMode.ALPHA
                ? 1
                : 0;
    }

    public static int annotationsArrayIncludesRuntimeAnnotation() {
        java.lang.annotation.Annotation[] annotations = AnnotationTest.class.getAnnotations();
        return annotations.length == 1 && annotations[0] instanceof DukeAnno ? 1 : 0;
    }
}
