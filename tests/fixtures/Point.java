public class Point {
    int x;
    int y;

    Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    int sum() {
        return x + y;
    }

    static int sumPoints(int ax, int ay, int bx, int by) {
        Point a = new Point(ax, ay);
        Point b = new Point(bx, by);
        return a.sum() + b.sum();
    }
}
