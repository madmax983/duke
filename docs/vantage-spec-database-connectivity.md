# 🔭 Vantage: Spec for JDBC Database Connectivity Support

## 👤 User Story
"As a Backend Engineer building data-intensive applications on Duke, I want the VM to support standard Java Database Connectivity (JDBC), so that my applications can query, insert, and manage data in relational databases without requiring proprietary data access layers."

## ❓ The "So What?"
What business problem does this solve?
Enterprise software is fundamentally about manipulating data. A runtime without database connectivity is effectively a sandbox. Currently, developers cannot connect their Java applications running on Duke to standard databases like PostgreSQL, MySQL, or Oracle. Without JDBC support, Duke cannot run the vast majority of real-world backend services, enterprise applications, or data processing pipelines. Supporting the `java.sql` API and the underlying networking prerequisites proves Duke is a production-ready runtime capable of fulfilling a core enterprise requirement: talking to a database.

## 📈 Metric Definition
Success = A user can deploy a standard Java application containing a JDBC driver (e.g., PostgreSQL JDBC Driver) on Duke, establish a `java.sql.Connection` to a remote database, execute a `SELECT 1` query via a `java.sql.Statement`, and retrieve the result through a `java.sql.ResultSet` without the VM panicking or throwing `UnsatisfiedLinkError`.

## 🔍 Gap Analysis
- **Current State:** Duke can execute basic computational bytecode, file I/O, and some foundational networking. However, it lacks support for the core `java.sql` interfaces and the specific native networking and security (TLS/SSL) primitives often required by modern JDBC drivers to establish secure connections.
- **Market/Standard Lib:** Standard JVMs provide the `java.sql` and `javax.sql` packages, relying on underlying socket implementations (`java.net.Socket`) and often secure sockets (`javax.net.ssl.SSLSocket`) which the drivers use to implement database-specific wire protocols.
- **The Gap:** Duke needs to ensure that the core networking stack is robust enough to handle high-throughput binary protocols (like Postgres wire protocol) and provide the necessary standard library classes (`java.sql.*`, `javax.sql.*`) so that third-party JDBC drivers can be loaded and executed.

## ✅ Acceptance Criteria
- Must support loading third-party JDBC drivers via `java.sql.DriverManager` or the modern `ServiceLoader` mechanism.
- Must provide the essential `java.sql` interfaces (Connection, Statement, PreparedStatement, ResultSet, SQLException) either by mapping to OpenJDK classes or synthesizing them.
- Must ensure the underlying networking primitives (`java.net.Socket`, streams) correctly support the binary read/write operations performed by standard JDBC drivers (e.g., PostgreSQL driver).
- Must handle connection timeouts and network errors gracefully, surfacing them as `SQLException`s rather than VM panics.

## 🚫 Out of Scope
- Writing custom JDBC drivers for Duke.
- Connection pooling implementations (e.g., HikariCP) — we only need to support the runtime requirements for them to work, not build the pool itself.
- ORM frameworks (e.g., Hibernate, JPA) — while they should eventually work if JDBC works, debugging ORM-specific reflection/proxying issues is secondary to raw JDBC connectivity.
