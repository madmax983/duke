use crate::Heap;
use duke_runtime::{VmError, VmResult};
use std::io::{Read, Write};

#[derive(Debug)]
/// A handle for a native host process managed by the VM.
///
/// Used for Java's `Runtime.exec` and related API implementations.
pub struct HostProcessHandle {
    child: std::process::Child,
    exit_code: Option<i32>,
}

/// Identifying file descriptors associated with a spawned native process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnedProcessIds {
    /// The unique process identifier.
    pub process_id: i32,
    /// The file descriptor ID for the process's standard input.
    pub stdin_id: i32,
    /// The file descriptor ID for the process's standard output.
    pub stdout_id: i32,
    /// The file descriptor ID for the process's standard error.
    pub stderr_id: i32,
}

#[derive(Debug)]
/// A handle to a native file managed by the VM on behalf of Java I/O classes.
pub enum HostFileHandle {
    /// A file opened for reading.
    Reader(std::fs::File),
    /// A file opened for writing.
    Writer(std::fs::File),
    /// A TCP server socket waiting for incoming connections.
    TcpListener(std::net::TcpListener),
    /// The read half of an accepted or connected TCP socket.
    SocketReader(std::net::TcpStream),
    /// The write half of an accepted or connected TCP socket.
    SocketWriter(std::net::TcpStream),
    /// An opened ZIP/JAR archive (parsed and indexed).
    ZipArchive(duke_loader::ZipReader),
    /// An in-memory byte buffer (e.g. decompressed ZIP entry for `InputStream`).
    ByteBuffer(std::io::Cursor<Vec<u8>>),
    /// An owned child process plus any cached exit status.
    Process(HostProcessHandle),
    /// The stdout pipe of a child process.
    ProcessStdout(std::process::ChildStdout),
    /// The stderr pipe of a child process.
    ProcessStderr(std::process::ChildStderr),
    /// The stdin pipe of a child process.
    ProcessStdin(std::process::ChildStdin),
}

impl Heap {
    /// Opens an input file on the host OS.
    ///
    /// # Errors
    /// Returns `VmError::JavaException` if the file does not exist or an IO error occurs.
    pub fn open_host_input_file(&mut self, path: &std::path::Path) -> VmResult<i32> {
        let file = std::fs::File::open(path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => VmError::JavaException {
                class_name: "java/io/FileNotFoundException".to_string(),
            },
            _ => VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            },
        })?;
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files.insert(id, HostFileHandle::Reader(file));
        Ok(id)
    }

    /// Opens an output file on the host OS.
    ///
    /// # Errors
    /// Returns `VmError::JavaException` if the file cannot be created.
    pub fn open_host_output_file(&mut self, path: &std::path::Path) -> VmResult<i32> {
        let file = std::fs::File::create(path).map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files.insert(id, HostFileHandle::Writer(file));
        Ok(id)
    }

    /// Reads a single byte from a host file.
    ///
    /// # Errors
    /// Returns `VmError::JavaException` if the file handle is invalid or an IO error occurs.
    pub fn read_host_file_byte(&mut self, id: i32) -> VmResult<i32> {
        let Some(handle) = self.host_files.get_mut(&id) else {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        };
        let reader: &mut dyn Read = match handle {
            HostFileHandle::Reader(f) => f,
            HostFileHandle::SocketReader(s) => s,
            HostFileHandle::ByteBuffer(cursor) => cursor,
            HostFileHandle::ProcessStdout(stdout) => stdout,
            HostFileHandle::ProcessStderr(stderr) => stderr,
            _ => {
                return Err(VmError::JavaException {
                    class_name: "java/io/IOException".into(),
                });
            }
        };
        let mut buf = [0_u8; 1];
        match reader.read(&mut buf) {
            Ok(0) => Ok(-1),
            Ok(_) => Ok(i32::from(buf[0])),
            Err(_) => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }

    /// Writes a single byte to a host file.
    ///
    /// # Errors
    /// Returns `VmError::JavaException` if the file handle is invalid or an IO error occurs.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn write_host_file_byte(&mut self, id: i32, value: i32) -> VmResult<()> {
        let Some(handle) = self.host_files.get_mut(&id) else {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        };
        let writer: &mut dyn Write = match handle {
            HostFileHandle::Writer(f) => f,
            HostFileHandle::SocketWriter(s) => s,
            HostFileHandle::ProcessStdin(stdin) => stdin,
            _ => {
                return Err(VmError::JavaException {
                    class_name: "java/io/IOException".into(),
                });
            }
        };
        writer
            .write_all(&[(value & 0xFF) as u8])
            .map_err(|_| VmError::JavaException {
                class_name: "java/io/IOException".into(),
            })
    }

    /// Closes a host file handle previously opened via the registry.
    ///
    /// Silently ignores invalid or already-closed file descriptors.
    pub fn close_host_file(&mut self, id: i32) {
        if id > 0 {
            self.host_files.remove(&id);
        }
    }

    const fn next_host_handle_id(&mut self) -> i32 {
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        id
    }

    /// Spawns a child process on the host OS with fully piped stdio.
    ///
    /// # Errors
    /// Returns `IOException` if the command is empty, cannot be spawned, or any
    /// of the expected pipes are unavailable.
    pub fn spawn_host_process(
        &mut self,
        command: &[String],
        cwd: Option<&std::path::Path>,
    ) -> VmResult<SpawnedProcessIds> {
        if command.is_empty() {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }

        let mut builder = std::process::Command::new(&command[0]);
        builder.args(&command[1..]);
        builder.stdin(std::process::Stdio::piped());
        builder.stdout(std::process::Stdio::piped());
        builder.stderr(std::process::Stdio::piped());
        if let Some(cwd) = cwd {
            builder.current_dir(cwd);
        }

        let mut child = builder.spawn().map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".into(),
        })?;
        let Some(stdin) = child.stdin.take() else {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        };
        let Some(stdout) = child.stdout.take() else {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        };
        let Some(stderr) = child.stderr.take() else {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        };

        let process_id = self.next_host_handle_id();
        let stdin_id = self.next_host_handle_id();
        let stdout_id = self.next_host_handle_id();
        let stderr_id = self.next_host_handle_id();

        self.host_files.insert(
            process_id,
            HostFileHandle::Process(HostProcessHandle {
                child,
                exit_code: None,
            }),
        );
        self.host_files
            .insert(stdin_id, HostFileHandle::ProcessStdin(stdin));
        self.host_files
            .insert(stdout_id, HostFileHandle::ProcessStdout(stdout));
        self.host_files
            .insert(stderr_id, HostFileHandle::ProcessStderr(stderr));

        Ok(SpawnedProcessIds {
            process_id,
            stdin_id,
            stdout_id,
            stderr_id,
        })
    }

    #[cfg(unix)]
    fn exit_status_code(status: std::process::ExitStatus) -> i32 {
        use std::os::unix::process::ExitStatusExt;

        status
            .code()
            .unwrap_or_else(|| status.signal().map_or(-1, |signal| 128 + signal))
    }

    #[cfg(not(unix))]
    fn exit_status_code(status: std::process::ExitStatus) -> i32 {
        status.code().unwrap_or(-1)
    }

    fn process_handle_mut(&mut self, id: i32) -> VmResult<&mut HostProcessHandle> {
        match self.host_files.get_mut(&id) {
            Some(HostFileHandle::Process(process)) => Ok(process),
            _ => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }

    /// Blocks until the child process exits and returns its cached exit code.
    ///
    /// # Errors
    /// Returns `IOException` if the process id is invalid or waiting fails.
    pub fn wait_host_process(&mut self, id: i32) -> VmResult<i32> {
        let process = self.process_handle_mut(id)?;
        if let Some(code) = process.exit_code {
            return Ok(code);
        }
        let status = process.child.wait().map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".into(),
        })?;
        let code = Self::exit_status_code(status);
        process.exit_code = Some(code);
        Ok(code)
    }

    /// Returns the child's exit code if it has already terminated.
    ///
    /// # Errors
    /// Returns `IOException` if the process id is invalid or querying status fails.
    pub fn try_host_process_exit_value(&mut self, id: i32) -> VmResult<Option<i32>> {
        let process = self.process_handle_mut(id)?;
        if let Some(code) = process.exit_code {
            return Ok(Some(code));
        }
        let status = process
            .child
            .try_wait()
            .map_err(|_| VmError::JavaException {
                class_name: "java/io/IOException".into(),
            })?;
        let Some(status) = status else {
            return Ok(None);
        };
        let code = Self::exit_status_code(status);
        process.exit_code = Some(code);
        Ok(Some(code))
    }

    /// Requests child termination if the process is still alive.
    ///
    /// # Errors
    /// Returns `IOException` if the process id is invalid or the kill operation fails.
    pub fn destroy_host_process(&mut self, id: i32) -> VmResult<()> {
        let process = self.process_handle_mut(id)?;
        if process.exit_code.is_some() {
            return Ok(());
        }
        if let Some(status) = process
            .child
            .try_wait()
            .map_err(|_| VmError::JavaException {
                class_name: "java/io/IOException".into(),
            })?
        {
            process.exit_code = Some(Self::exit_status_code(status));
            return Ok(());
        }
        process.child.kill().map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".into(),
        })
    }

    /// Opens a ZIP/JAR archive on the host OS, parses and indexes it.
    ///
    /// # Errors
    /// Returns `ZipException` if the file is not a valid ZIP, or
    /// `FileNotFoundException` if the path does not exist.
    pub fn open_host_zip(&mut self, path: &std::path::Path) -> VmResult<i32> {
        let reader = duke_loader::ZipReader::open(path).map_err(|err| match err {
            duke_loader::LoadError::Io { .. } => VmError::JavaException {
                class_name: "java/io/FileNotFoundException".to_string(),
            },
            _ => VmError::JavaException {
                class_name: "java/util/zip/ZipException".to_string(),
            },
        })?;
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files
            .insert(id, HostFileHandle::ZipArchive(reader));
        Ok(id)
    }

    /// Returns the number of entries in an opened ZIP archive.
    ///
    /// # Errors
    /// Returns `IOException` if the handle is invalid or not a ZIP archive.
    pub fn zip_entry_count(&self, id: i32) -> VmResult<usize> {
        match self.host_files.get(&id) {
            Some(HostFileHandle::ZipArchive(reader)) => Ok(reader.entry_count()),
            _ => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }

    /// Looks up a ZIP entry by name, returning a clone of its metadata.
    ///
    /// # Errors
    /// Returns `IOException` if the handle is invalid or not a ZIP archive.
    pub fn zip_get_entry_info(
        &self,
        id: i32,
        name: &str,
    ) -> VmResult<Option<duke_loader::ZipEntryInfo>> {
        match self.host_files.get(&id) {
            Some(HostFileHandle::ZipArchive(reader)) => Ok(reader.get_entry(name).cloned()),
            _ => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }

    /// Reads and decompresses a ZIP entry's bytes.
    ///
    /// # Errors
    /// Returns `ZipException` on decompression failure, `IOException` for invalid handles.
    pub fn zip_read_entry(&self, id: i32, name: &str) -> VmResult<Vec<u8>> {
        match self.host_files.get(&id) {
            Some(HostFileHandle::ZipArchive(reader)) => {
                reader.read_entry(name).map_err(|_| VmError::JavaException {
                    class_name: "java/util/zip/ZipException".into(),
                })
            }
            _ => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }

    /// Creates an in-memory byte buffer handle (for reading decompressed data
    /// as an `InputStream`).
    pub fn open_host_byte_buffer(&mut self, data: Vec<u8>) -> i32 {
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files
            .insert(id, HostFileHandle::ByteBuffer(std::io::Cursor::new(data)));
        id
    }

    /// Binds a TCP listener to the given address string (e.g. `"0.0.0.0:8080"`).
    ///
    /// # Errors
    /// Returns `BindException` if the address is already in use, `SocketException` for other errors.
    pub fn bind_server_socket(&mut self, addr: &str) -> VmResult<i32> {
        let listener = std::net::TcpListener::bind(addr).map_err(|err| match err.kind() {
            std::io::ErrorKind::AddrInUse => VmError::JavaException {
                class_name: "java/net/BindException".into(),
            },
            _ => VmError::JavaException {
                class_name: "java/net/SocketException".into(),
            },
        })?;
        let id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files
            .insert(id, HostFileHandle::TcpListener(listener));
        Ok(id)
    }

    /// Accepts one incoming connection on the given listener id.
    /// Returns `(reader_id, writer_id)` — two independent OS handles to the same socket.
    ///
    /// # Errors
    /// Returns `IOException` if the id is invalid or the accept fails.
    ///
    /// Note: Handle IDs are allocated with `saturating_add`; extremely long-running
    /// programs opening billions of handles would alias at `i32::MAX`. This is a
    /// known limitation shared with the file I/O implementation.
    pub fn accept_connection(&mut self, id: i32) -> VmResult<(i32, i32)> {
        // Validate that the handle exists and is a TcpListener.
        if !matches!(
            self.host_files.get(&id),
            Some(HostFileHandle::TcpListener(_))
        ) {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
        // Temporarily remove the listener to satisfy the borrow checker, then reinsert.
        let Some(HostFileHandle::TcpListener(listener)) = self.host_files.remove(&id) else {
            unreachable!()
        };
        let result = listener.accept();
        self.host_files
            .insert(id, HostFileHandle::TcpListener(listener));
        let (stream, _addr) = result.map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".into(),
        })?;
        let writer = stream.try_clone().map_err(|_| VmError::JavaException {
            class_name: "java/io/IOException".into(),
        })?;
        let reader_id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        let writer_id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files
            .insert(reader_id, HostFileHandle::SocketReader(stream));
        self.host_files
            .insert(writer_id, HostFileHandle::SocketWriter(writer));
        Ok((reader_id, writer_id))
    }

    /// Connects a TCP socket to the given address string (e.g. `"127.0.0.1:8080"`).
    /// Returns `(reader_id, writer_id)` — two independent OS handles to the same socket.
    ///
    /// # Errors
    /// Returns `ConnectException` if connection is refused, `SocketException` for other errors.
    ///
    /// Note: Handle IDs are allocated with `saturating_add`; extremely long-running
    /// programs opening billions of handles would alias at `i32::MAX`. This is a
    /// known limitation shared with the file I/O implementation.
    pub fn connect_socket(&mut self, addr: &str) -> VmResult<(i32, i32)> {
        let stream = std::net::TcpStream::connect(addr).map_err(|err| match err.kind() {
            std::io::ErrorKind::ConnectionRefused => VmError::JavaException {
                class_name: "java/net/ConnectException".into(),
            },
            _ => VmError::JavaException {
                class_name: "java/net/SocketException".into(),
            },
        })?;
        let writer = stream.try_clone().map_err(|_| VmError::JavaException {
            class_name: "java/net/SocketException".into(),
        })?;
        let reader_id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        let writer_id = self.next_host_file_id;
        self.next_host_file_id = self.next_host_file_id.saturating_add(1);
        self.host_files
            .insert(reader_id, HostFileHandle::SocketReader(stream));
        self.host_files
            .insert(writer_id, HostFileHandle::SocketWriter(writer));
        Ok((reader_id, writer_id))
    }

    /// Returns the local port of a bound server socket.
    ///
    /// # Errors
    /// Returns `IOException` if the id is invalid.
    pub fn server_socket_local_port(&self, id: i32) -> VmResult<i32> {
        match self.host_files.get(&id) {
            Some(HostFileHandle::TcpListener(l)) => l
                .local_addr()
                .map(|addr| i32::from(addr.port()))
                .map_err(|_| VmError::JavaException {
                    class_name: "java/io/IOException".into(),
                }),
            _ => Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_host_file_operations() {
        let mut heap = Heap::new();
        // Use an existing test file from duke-loader tests if possible, or create a dummy
        let dummy_path = std::env::temp_dir().join("test_host_file_ops.txt");
        std::fs::write(&dummy_path, b"test").unwrap();

        let fd = heap.open_host_input_file(&dummy_path).unwrap();
        let b = heap.read_host_file_byte(fd).unwrap();
        assert_eq!(b, i32::from(b't'));
        heap.close_host_file(fd);

        // open_host_output_file
        let out_fd = heap.open_host_output_file(&dummy_path).unwrap();
        heap.write_host_file_byte(out_fd, i32::from(b'A')).unwrap();
        heap.close_host_file(out_fd);

        let content = std::fs::read_to_string(&dummy_path).unwrap();
        assert_eq!(content, "A");

        std::fs::remove_file(dummy_path).unwrap();
    }

    #[test]
    fn should_handle_open_read_write_close_cycle() {
        let mut heap = Heap::new();
        let dummy_path = std::env::temp_dir().join("test_host_file_ops2.txt");

        let out_fd = heap.open_host_output_file(&dummy_path).unwrap();
        heap.write_host_file_byte(out_fd, i32::from(b'A')).unwrap();
        heap.write_host_file_byte(out_fd, i32::from(b'B')).unwrap();
        heap.close_host_file(out_fd);

        let in_fd = heap.open_host_input_file(&dummy_path).unwrap();
        let b1 = heap.read_host_file_byte(in_fd).unwrap();
        let b2 = heap.read_host_file_byte(in_fd).unwrap();
        let b3 = heap.read_host_file_byte(in_fd).unwrap(); // EOF
        assert_eq!(b1, i32::from(b'A'));
        assert_eq!(b2, i32::from(b'B'));
        assert_eq!(b3, -1);

        heap.close_host_file(in_fd);
        std::fs::remove_file(dummy_path).unwrap();
    }

    #[test]
    fn open_host_input_file_not_found() {
        let mut heap = Heap::new();
        let dummy_path = std::env::temp_dir().join("does_not_exist_12345.txt");
        let result = heap.open_host_input_file(&dummy_path);
        assert!(
            matches!(result, Err(duke_runtime::VmError::JavaException { class_name }) if class_name == "java/io/FileNotFoundException")
        );
    }

    #[test]
    fn spawn_host_process_empty_command() {
        let mut heap = Heap::new();
        let result = heap.spawn_host_process(&[], None);
        assert!(
            matches!(result, Err(duke_runtime::VmError::JavaException { class_name }) if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn spawn_host_process_invalid_command() {
        let mut heap = Heap::new();
        let result = heap.spawn_host_process(&["/does/not/exist/executable".to_string()], None);
        assert!(
            matches!(result, Err(duke_runtime::VmError::JavaException { class_name }) if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn read_write_invalid_host_file_handle() {
        let mut heap = Heap::new();
        assert!(
            matches!(heap.read_host_file_byte(999), Err(duke_runtime::VmError::JavaException { class_name }) if class_name == "java/io/IOException")
        );
        assert!(
            matches!(heap.write_host_file_byte(999, 10), Err(duke_runtime::VmError::JavaException { class_name }) if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn process_wait_and_destroy_cycle() {
        let mut heap = Heap::new();
        // Spawning an executable command that succeeds immediately. We use "true" (cross-platform way via sh/cmd)
        let cmd = if cfg!(windows) {
            vec!["cmd".to_string(), "/C".to_string(), "exit 0".to_string()]
        } else {
            vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()]
        };
        let p_ids = heap.spawn_host_process(&cmd, None).unwrap();

        let wait_result = heap.wait_host_process(p_ids.process_id).unwrap();
        assert_eq!(wait_result, 0);

        // Waiting again should return the cached exit code
        let wait_result2 = heap.wait_host_process(p_ids.process_id).unwrap();
        assert_eq!(wait_result2, 0);

        // try_host_process_exit_value should also return the cached code
        let try_val = heap.try_host_process_exit_value(p_ids.process_id).unwrap();
        assert_eq!(try_val, Some(0));

        // destroy on a finished process is a no-op
        heap.destroy_host_process(p_ids.process_id).unwrap();
    }

    #[test]
    fn process_destroy_running() {
        let mut heap = Heap::new();
        let cmd = if cfg!(windows) {
            vec!["cmd".to_string(), "/C".to_string(), "pause".to_string()]
        } else {
            vec!["cat".to_string()]
        };
        let p_ids = heap.spawn_host_process(&cmd, None).unwrap();
        heap.destroy_host_process(p_ids.process_id).unwrap();
    }

    #[test]
    fn should_return_error_when_waiting_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.wait_host_process(999).unwrap_err();
        assert!(matches!(err, duke_runtime::VmError::JavaException { .. }));
    }

    #[test]
    fn should_return_error_when_trying_exit_value_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.try_host_process_exit_value(999).unwrap_err();
        assert!(matches!(err, duke_runtime::VmError::JavaException { .. }));
    }

    #[test]
    fn should_return_error_when_destroying_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.destroy_host_process(999).unwrap_err();
        assert!(matches!(err, duke_runtime::VmError::JavaException { .. }));
    }

    #[test]
    fn socket_operations() {
        let mut heap = Heap::new();
        // connect to an invalid host should fail
        let res = heap.connect_socket("invalid.host.local:12345");
        assert!(res.is_err());

        // test operations on a server socket using a random port (0)
        let server_fd = heap.bind_server_socket("127.0.0.1:0").unwrap();
        let port = heap.server_socket_local_port(server_fd).unwrap();

        // Cannot accept right away in a blocking manner easily without hanging or starting a client thread,
        // but we can try opening a client to it
        let client_fd = heap.connect_socket(&format!("127.0.0.1:{port}")).unwrap();

        // closing sockets
        heap.close_host_file(client_fd.0);
        heap.close_host_file(client_fd.1);
        heap.close_host_file(server_fd);
    }
}


#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn open_host_byte_buffer_and_read() {
        let mut heap = Heap::new();
        let id = heap.open_host_byte_buffer(vec![10, 20, 30]);
        assert_eq!(heap.read_host_file_byte(id).unwrap(), 10);
        assert_eq!(heap.read_host_file_byte(id).unwrap(), 20);
        assert_eq!(heap.read_host_file_byte(id).unwrap(), 30);
        assert_eq!(heap.read_host_file_byte(id).unwrap(), -1);
    }

    #[test]
    fn test_try_host_process_exit_value_running() {
        let mut heap = Heap::new();
        // Spawning an executable command that succeeds immediately. We use "true" (cross-platform way via sh/cmd)
        let cmd = if cfg!(windows) {
            vec!["cmd".to_string(), "/C".to_string(), "pause".to_string()]
        } else {
            vec!["cat".to_string()]
        };
        let p_ids = heap.spawn_host_process(&cmd, None).unwrap();
        // Should return None if still running
        let try_val = heap.try_host_process_exit_value(p_ids.process_id).unwrap();
        assert_eq!(try_val, None);

        heap.destroy_host_process(p_ids.process_id).unwrap();
    }

    #[test]
    fn spawn_host_process_with_cwd() {
        let mut heap = Heap::new();
        let cmd = if cfg!(windows) {
            vec!["cmd".to_string(), "/C".to_string(), "cd".to_string()]
        } else {
            vec!["pwd".to_string()]
        };
        let cwd = std::env::temp_dir();
        let p_ids = heap.spawn_host_process(&cmd, Some(&cwd)).unwrap();
        heap.wait_host_process(p_ids.process_id).unwrap();
    }
}

    #[test]
    fn spawn_host_process_io_error() {
        let mut heap = Heap::new();
        // Should trigger an io error and map to IOException
        let result = heap.spawn_host_process(&["/definitely/invalid/command".to_string()], None);
        assert!(matches!(result, Err(VmError::JavaException { class_name }) if class_name == "java/io/IOException"));
    }
