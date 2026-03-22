import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

public final class ProcessManagementTest {
    private ProcessManagementTest() {
    }

    private static boolean isPowerShell(String launcher) {
        return launcher.toLowerCase().contains("powershell");
    }

    private static String[] shellCommand(String launcher, String script) {
        if (isPowerShell(launcher)) {
            return new String[] { launcher, "-NoProfile", "-NonInteractive", "-Command", script };
        }
        return new String[] { launcher, "-c", script };
    }

    private static String[] childCommand(String launcher, String classpath, String mode) {
        if (mode.equals("stdout")) {
            if (isPowerShell(launcher)) {
                return shellCommand(launcher, "[Console]::Out.Write('duke-out')");
            }
            return shellCommand(launcher, "printf duke-out");
        }
        if (mode.equals("cwd-probe")) {
            if (isPowerShell(launcher)) {
                return shellCommand(
                        launcher,
                        "if (Test-Path tests -PathType Container) { [Console]::Out.Write('cwd-ok') } else { [Console]::Out.Write('cwd-bad') }"
                );
            }
            return shellCommand(launcher, "if [ -d tests ]; then printf cwd-ok; else printf cwd-bad; fi");
        }
        if (mode.equals("stderr")) {
            if (isPowerShell(launcher)) {
                return shellCommand(launcher, "[Console]::Error.Write('duke-err')");
            }
            return shellCommand(launcher, "printf duke-err >&2");
        }
        if (mode.equals("stdin-sum")) {
            if (isPowerShell(launcher)) {
                return shellCommand(
                        launcher,
                        "$sum = 0; $stdin = [Console]::OpenStandardInput(); while (($b = $stdin.ReadByte()) -ne -1) { $sum += $b }; [Console]::Out.Write(\"sum=$sum\")"
                );
            }
            return shellCommand(
                    launcher,
                    "exec python3 -c \"import sys; data=sys.stdin.buffer.read(); sys.stdout.write('sum=%d' % sum(data))\""
            );
        }
        if (mode.equals("sleep")) {
            if (isPowerShell(launcher)) {
                return shellCommand(launcher, "Start-Sleep -Seconds 10");
            }
            return shellCommand(launcher, "exec sleep 10");
        }
        return shellCommand(launcher, "exit 9");
    }

    private static int streamContains(InputStream in, int[] needle) throws Exception {
        int matched = 0;
        int next;
        while ((next = in.read()) != -1) {
            if (next == needle[matched]) {
                matched++;
                if (matched == needle.length) {
                    return 1;
                }
                continue;
            }
            matched = next == needle[0] ? 1 : 0;
        }
        return 0;
    }

    public static int spawnAndReadStdout(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "stdout"))
                .directory(new File(workDir))
                .start();
        InputStream in = process.getInputStream();
        int contains = streamContains(in, new int[] { 'd', 'u', 'k', 'e', '-', 'o', 'u', 't' });
        in.close();
        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
    }

    public static int runtimeExecReadsStdout(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = Runtime.getRuntime().exec(
                childCommand(javaCmd, classpath, "stdout"),
                null,
                new File(workDir)
        );
        InputStream in = process.getInputStream();
        int contains = streamContains(in, new int[] { 'd', 'u', 'k', 'e', '-', 'o', 'u', 't' });
        in.close();
        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
    }

    public static int processBuilderAppliesWorkingDirectory(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "cwd-probe"))
                .directory(new File(workDir))
                .start();
        InputStream in = process.getInputStream();
        int contains = streamContains(in, new int[] { 'c', 'w', 'd', '-', 'o', 'k' });
        in.close();
        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
    }

    public static int runtimeExecAppliesWorkingDirectory(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = Runtime.getRuntime().exec(
                childCommand(javaCmd, classpath, "cwd-probe"),
                null,
                new File(workDir)
        );
        InputStream in = process.getInputStream();
        int contains = streamContains(in, new int[] { 'c', 'w', 'd', '-', 'o', 'k' });
        in.close();
        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
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
        int contains = streamContains(in, new int[] { 's', 'u', 'm', '=', '1', '2' });
        in.close();

        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
    }

    public static int readErrorStream(String javaCmd, String classpath, String workDir) throws Exception {
        Process process = new ProcessBuilder(childCommand(javaCmd, classpath, "stderr"))
                .directory(new File(workDir))
                .start();
        InputStream err = process.getErrorStream();
        int contains = streamContains(err, new int[] { 'd', 'u', 'k', 'e', '-', 'e', 'r', 'r' });
        err.close();
        int exit = process.waitFor();
        return contains == 1 && exit == process.exitValue() ? 1 : 0;
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
