import java.net.*;
import java.io.*;

public class NetworkingTest {

    // Bind a ServerSocket to port 0, return the actual local port (should be > 0)
    static int bindAndGetPort() throws Exception {
        ServerSocket ss = new ServerSocket(0);
        int port = ss.getLocalPort();
        ss.close();
        return port;
    }

    // Try to connect to a port where nothing is listening — should catch ConnectException
    // Returns 1 if ConnectException was caught, 0 if connection unexpectedly succeeded
    static int connectRefused(int port) {
        try {
            Socket s = new Socket("127.0.0.1", port);
            s.close();
            return 0;
        } catch (ConnectException e) {
            return 1;
        } catch (Exception e) {
            return 1; // any exception means "refused" for our purposes
        }
    }

    // Server-side: bind, accept one connection, read one byte, return its value
    // The Rust test thread will connect and write the byte
    static int acceptAndReadByte(int port) throws Exception {
        ServerSocket ss = new ServerSocket(port);
        Socket client = ss.accept();
        InputStream in = client.getInputStream();
        int b = in.read();
        in.close();
        client.close();
        ss.close();
        return b;
    }

    // Client-side: connect to port, write one byte, return 1 on success
    // The Rust test thread will listen on that port
    static int connectAndWriteByte(int port, int value) throws Exception {
        Socket s = new Socket("127.0.0.1", port);
        OutputStream out = s.getOutputStream();
        out.write(value);
        out.close();
        s.close();
        return 1;
    }

    // Echo server: accept one connection, read `count` bytes, write them back, return count
    // The Rust test thread will connect, send bytes, and read them back
    static int acceptEchoAndCount(int port, int count) throws Exception {
        ServerSocket ss = new ServerSocket(port);
        Socket client = ss.accept();
        InputStream in = client.getInputStream();
        OutputStream out = client.getOutputStream();
        for (int i = 0; i < count; i++) {
            int b = in.read();
            out.write(b);
        }
        out.close();
        in.close();
        client.close();
        ss.close();
        return count;
    }

    // Client-side echo check: connect, write count bytes (1..count), read them back,
    // return count if all match
    static int connectAndEchoCheck(int port, int count) throws Exception {
        Socket s = new Socket("127.0.0.1", port);
        OutputStream out = s.getOutputStream();
        InputStream in = s.getInputStream();
        for (int i = 1; i <= count; i++) {
            out.write(i);
            int b = in.read();
            if (b != i) {
                s.close();
                return -1;
            }
        }
        s.close();
        return count;
    }
}
