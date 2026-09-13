public class P02Sealed {
    sealed interface Shape permits Circle, Rect, Triangle {}
    record Circle(double r) implements Shape {}
    record Rect(double w, double h) implements Shape {}
    final class Triangle implements Shape { double b, h; Triangle(double b, double h) { this.b = b; this.h = h; } }

    sealed static class Vehicle permits Car, Truck {}
    static non-sealed class Car extends Vehicle {}
    static final class Truck extends Vehicle {}

    public static void main(String[] args) {
        System.out.println("shape-sealed=" + Shape.class.isSealed());
        StringBuilder sb = new StringBuilder();
        for (Class<?> c : Shape.class.getPermittedSubclasses()) sb.append(c.getSimpleName()).append(";");
        System.out.println("permitted=" + sb);
        System.out.println("vehicle-sealed=" + Vehicle.class.isSealed());
        System.out.println("car-sealed=" + Car.class.isSealed());

        // exhaustive switch over sealed, no default needed
        Shape s = new Rect(3, 4);
        String desc = switch (s) {
            case Circle c -> "circle";
            case Rect r -> "rect";
            case Triangle t -> "triangle";
        };
        System.out.println("exhaustive=" + desc);

        Shape s2 = new Circle(1.0);
        double area = switch (s2) {
            case Circle(var r) -> Math.PI * r * r;
            case Rect(var w, var h) -> w * h;
            case Triangle t -> t.b * t.h / 2;
        };
        System.out.println("area-circle=" + (Math.abs(area - Math.PI) < 1e-9));

        Vehicle v = new Truck();
        String kind = switch (v) {
            case Car ignored -> "car";
            case Truck ignored -> "truck";
            default -> "other-vehicle"; // non-sealed Car breaks exhaustiveness
        };
        System.out.println("vehicle-switch=" + kind);
    }
}
