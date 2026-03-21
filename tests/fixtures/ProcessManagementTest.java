import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

public final class ProcessManagementTest {
    private static final int STDOUT_SUM = 814;
    private static final int STDERR_SUM = 799;

    private ProcessManagementTest() {
    }

    private static String[] childCommand(String javaCmd, String classpath, String mode) {
        return new String[] { javaCmd, "-cp", classpath, "ProcessChildMain", mode };
    }

    private static int readAllSum(InputStream in) throws Exception {
        int sum = 0;
        int next;
        while ((next = in.read()) != -1) {
            sum += next;
        }
        return sum;
    }

    public static int spawnAndReadStdout(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "stdout"))
                .directory(new File(workDir))
                .start();
        InputStream in = process.getInputStream();
        int sum = readAllSum(in);
        in.close();
        int exit = process.waitFor();
        return sum == STDOUT_SUM && exit == 7 && process.exitValue() == 7 ? 1 : 0;
    }

    public static int runtimeExecReadsStdout(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = Runtime.getRuntime().exec(
                childCommand(javaCmd, classpath, "stdout"),
                null,
                new File(workDir)
        );
        InputStream in = process.getInputStream();
        int sum = readAllSum(in);
        in.close();
        int exit = process.waitFor();
        return sum == STDOUT_SUM && exit == 7 && process.exitValue() == 7 ? 1 : 0;
    }

    public static int pipeStdinToChild(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "stdin-sum"))
                .directory(new File(workDir))
                .start();
        OutputStream out = process.getOutputStream();
        out.write(3);
        out.write(4);
        out.write(5);
        out.close();

        InputStream in = process.getInputStream();
        int first = in.read();
        int second = in.read();
        in.close();

        int exit = process.waitFor();
        return first == 12 && second == -1 && exit == 4 ? 1 : 0;
    }

    public static int readErrorStream(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "stderr"))
                .directory(new File(workDir))
                .start();
        InputStream err = process.getErrorStream();
        int sum = readAllSum(err);
        err.close();
        int exit = process.waitFor();
        return sum == STDERR_SUM && exit == 5 ? 1 : 0;
    }

    public static int destroySleepingChild(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "sleep"))
                .directory(new File(workDir))
                .start();
        int sawIllegalThreadState = 0;
        try {
            process.exitValue();
        } catch (IllegalThreadStateException e) {
            sawIllegalThreadState = 1;
        }
        process.destroy();
        int exit = process.waitFor();
        return sawIllegalThreadState == 1 && exit != 0 ? 1 : 0;
    }

    public static int missingExecutableRaisesIoException() {
        try {
            new ProcessBuilder(new String[] { "duke-definitely-not-a-real-executable-phase32" }).start();
            return 0;
        } catch (IOException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }
}

final class ProcessChildMain {
    private ProcessChildMain() {
    }

    public static void main(String[] args) throws Exception {
        String mode = args[0];

        if (mode.equals("stdout")) {
            if (new File("tests").isDirectory()) {
                System.out.print("duke-out");
                System.exit(7);
            }
            System.exit(2);
        }

        if (mode.equals("stderr")) {
            System.err.print("duke-err");
            System.exit(5);
        }

        if (mode.equals("stdin-sum")) {
            int sum = 0;
            int next;
            while ((next = System.in.read()) != -1) {
                sum += next;
            }
            System.out.write(sum);
            System.out.flush();
            System.exit(4);
        }

        if (mode.equals("sleep")) {
            Thread.sleep(10_000L);
            System.exit(0);
        }

        System.exit(9);
    }
}
