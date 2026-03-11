/**
 * Exercises the lambda SAM native-fallback dispatch path (Site 5).
 *
 * {@code s::length} is a captured-instance virtual method reference
 * (REF_invokeVirtual, impl_kind=5).  The captured {@code s} is stored in
 * the lambda proxy's fields[0] and {@code captured_count=1}; the SAM
 * {@code IntSupplier.get()} takes no additional parameters.
 *
 * When {@code sup.get()} is called via invokeinterface, the interpreter
 * builds {@code impl_args=[captured_s]} and resolves
 * impl_class="java/lang/String", impl_method="length", impl_kind=5.
 *
 * Because {@code String.length} has no bytecode in Duke's class registry
 * (it is a synthetic native), {@code resolve_method_in_hierarchy} returns
 * {@code None} and execution falls through to the native-registry lookup —
 * exactly the lambda SAM native-fallback arm (Site 5).
 *
 * The method returns an {@code int} directly (no boxing), so no
 * {@code checkcast} or auto-unboxing is needed.
 */
public class LambdaCallbackTest {
    interface IntSupplier { int get(); }

    public static int capturedLengthViaMethodRef(String s) {
        IntSupplier sup = s::length;
        return sup.get();
    }
}
