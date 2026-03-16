# 🔭 Vantage: Spec for Basic Networking (`java.net.Socket`, `java.net.ServerSocket`)

## 👤 User Story
"As a Backend Developer running on Duke, I want to open network connections and listen for incoming requests using standard Java sockets, so that my applications can communicate with other services, databases, and clients over the internet."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke applications operate in a local sandbox without the ability to communicate externally over the network. Modern software is inherently networked; applications act as web servers, consume REST APIs, query databases, or participate in distributed systems. Without network support, Duke cannot host anything beyond localized scripts or batch processors. Adding basic networking unlocks the vast majority of server-side Java workloads (e.g., HTTP servers, microservices, and database clients), transitioning Duke from an isolated compute engine into a viable platform for internet-facing applications.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully start a `ServerSocket` on a local port, accept an incoming connection, read an HTTP request using `Socket.getInputStream()`, write an HTTP response using `Socket.getOutputStream()`, and successfully close the connection without resource leaks or VM panics.

## 🔍 Gap Analysis
- **Current State:** Duke has no implementation for any `java.net` classes. Any attempt to use network sockets will fail due to missing native method implementations.
- **Market/Standard Lib:** Standard Java relies on `java.net.Socket` for client-side TCP connections and `java.net.ServerSocket` for listening to incoming TCP connections. These wrap host OS socket handles.
- **The Gap:** We need native bridge implementations that map Java's socket lifecycle (bind, listen, accept, connect, read, write, close) to Rust's standard library `std::net::TcpStream` and `std::net::TcpListener`.

## ✅ Acceptance Criteria
- Must support creating and binding a `ServerSocket` to a specific port.
- Must implement `ServerSocket.accept()` to block and return a connected client `Socket`.
- Must support creating a client `Socket` and connecting to a remote host/port.
- Must implement basic I/O via `Socket.getInputStream()` and `Socket.getOutputStream()`.
- Must integrate stream reading and writing with the byte-level I/O patterns established in File I/O.
- Must implement `close()` on sockets and streams, properly releasing host OS resources.
- Must raise standard Java exceptions (e.g., `java.net.BindException`, `java.net.ConnectException`, `java.io.IOException`) for network failures like "address in use" or "connection refused".

## 🚫 Out of Scope
- UDP Networking (`java.net.DatagramSocket`).
- Non-blocking I/O (`java.nio.channels`).
- Advanced socket options (e.g., `SO_LINGER`, `TCP_NODELAY`, `SO_RCVBUF`).
- SSL/TLS encryption (`javax.net.ssl`).
- Asynchronous networking.
