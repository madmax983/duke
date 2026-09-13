import java.lang.annotation.*;
import java.lang.reflect.*;

public class P11Annotations {
    @Retention(RetentionPolicy.RUNTIME)
    @Target({ElementType.TYPE, ElementType.METHOD, ElementType.FIELD,
             ElementType.RECORD_COMPONENT, ElementType.PARAMETER})
    @interface Tag { String value(); int n() default 7; }

    @Tag("class-tag")
    static class Marked {
        @Tag(value = "field-tag", n = 1)
        public String f = "x";
        @Tag("method-tag")
        public void m(@Tag("param-tag") String p) {}
    }

    record R(@Tag("comp-tag") int x, String y) {}

    public static void main(String[] args) throws Exception {
        System.out.println("class=" + Marked.class.getAnnotation(Tag.class).value());
        Field f = Marked.class.getField("f");
        Tag ft = f.getAnnotation(Tag.class);
        System.out.println("field=" + ft.value() + "," + ft.n());
        Method m = Marked.class.getMethod("m", String.class);
        System.out.println("method=" + m.getAnnotation(Tag.class).value());
        Annotation[][] pa = m.getParameterAnnotations();
        System.out.println("param=" + ((Tag) pa[0][0]).value());

        // annotations on record components propagate to field/accessor/param
        RecordComponent rc = R.class.getRecordComponents()[0];
        System.out.println("record-component=" + rc.getAnnotation(Tag.class).value());
        System.out.println("record-field=" + R.class.getDeclaredField("x").getAnnotation(Tag.class).value());
        System.out.println("record-accessor=" + R.class.getMethod("x").getAnnotation(Tag.class).value());

        // inherited annotations
        System.out.println("annotation-present=" + Marked.class.isAnnotationPresent(Tag.class));
        System.out.println("annotations-count=" + Marked.class.getAnnotations().length);
    }
}
