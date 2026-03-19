import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;

public class FileIoTest {
    private static final int FILE_EXISTS = 1;
    private static final int FILE_IS_FILE = 2;
    private static final int FILE_IS_DIRECTORY = 4;
    private static final int DIR_EXISTS = 8;
    private static final int DIR_IS_FILE = 16;
    private static final int DIR_IS_DIRECTORY = 32;
    private static final int MISSING_EXISTS = 64;
    private static final int MISSING_IS_FILE = 128;
    private static final int MISSING_IS_DIRECTORY = 256;

    public static int inspectKinds(String file, String dir, String missing) {
        int mask = 0;

        File f = new File(file);
        if (f.exists()) mask |= FILE_EXISTS;
        if (f.isFile()) mask |= FILE_IS_FILE;
        if (f.isDirectory()) mask |= FILE_IS_DIRECTORY;

        File d = new File(dir);
        if (d.exists()) mask |= DIR_EXISTS;
        if (d.isFile()) mask |= DIR_IS_FILE;
        if (d.isDirectory()) mask |= DIR_IS_DIRECTORY;

        File m = new File(missing);
        if (m.exists()) mask |= MISSING_EXISTS;
        if (m.isFile()) mask |= MISSING_IS_FILE;
        if (m.isDirectory()) mask |= MISSING_IS_DIRECTORY;

        return mask;
    }

    public static int readAllAndSum(String path) throws Exception {
        int sum = 0;
        try (FileInputStream in = new FileInputStream(path)) {
            int next;
            while ((next = in.read()) != -1) {
                sum += next;
            }
        }
        return sum;
    }

    public static int copyAndCount(String from, String to) throws Exception {
        int count = 0;
        try (FileInputStream in = new FileInputStream(from);
             FileOutputStream out = new FileOutputStream(to)) {
            int next;
            while ((next = in.read()) != -1) {
                out.write(next);
                count++;
            }
        }
        return count;
    }

    public static int copyWithBuffer(String from, String to) throws Exception {
        byte[] buffer = new byte[4];
        try (FileInputStream in = new FileInputStream(from);
             FileOutputStream out = new FileOutputStream(to)) {
            int count = in.read(buffer);
            if (count > 0) {
                out.write(buffer);
            }
            return count;
        }
    }

    public static int missingFile(String path) {
        try (FileInputStream in = new FileInputStream(path)) {
            return 0;
        } catch (FileNotFoundException e) {
            return 1;
        } catch (Exception e) {
            return -1;
        }
    }

    public static int readAfterClose(String path) throws Exception {
        try (FileInputStream in = new FileInputStream(path)) {
            in.close();
            try {
                in.read();
                return 0;
            } catch (IOException e) {
                return 1;
            }
        }
    }

    public static int writeAfterClose(String path) throws Exception {
        try (FileOutputStream out = new FileOutputStream(path)) {
            out.close();
            try {
                out.write(42);
                return 0;
            } catch (IOException e) {
                return 1;
            }
        }
    }
}
