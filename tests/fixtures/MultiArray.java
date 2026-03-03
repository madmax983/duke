public class MultiArray {
    /** Creates a 2x3 int[][] and returns sum of all elements after init. */
    public static int sum2d() {
        int[][] grid = new int[2][3];
        int val = 1;
        for (int i = 0; i < 2; i++) {
            for (int j = 0; j < 3; j++) {
                grid[i][j] = val++;
            }
        }
        // grid = {{1,2,3},{4,5,6}} -> sum = 21
        int sum = 0;
        for (int i = 0; i < 2; i++) {
            for (int j = 0; j < 3; j++) {
                sum += grid[i][j];
            }
        }
        return sum; // 21
    }

    /** Returns dimensions: outer length * inner length. */
    public static int dimensions() {
        int[][] grid = new int[3][4];
        return grid.length * grid[0].length; // 12
    }
}
