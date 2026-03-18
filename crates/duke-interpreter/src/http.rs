use duke_runtime::{Slot, VmError, VmResult};
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::{ClassContext, ClassRegistry, MethodEntry};

/// A simple, additive native HTTP client prototype.
/// Enables `duke.net.HttpClient.fetch(String)` for basic data scraping.
pub fn register(registry: &mut ClassRegistry) {
    let mut http_client_ctx = ClassContext {
        class_name: "duke/net/HttpClient".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };

    // Make the native method discoverable by the JVM.
    // JVM signature: public static native String fetch(String url);
    http_client_ctx.methods.push(MethodEntry {
        name: "fetch".to_string(),
        descriptor: "(Ljava/lang/String;)Ljava/lang/String;".to_string(),
        instructions: Vec::new(), // Native methods have no bytecode
        max_stack: 0,
        max_locals: 1, // args[0] is the url parameter
        exception_table: Vec::new(),
        pc_to_idx: std::sync::Arc::new(std::collections::HashMap::new()),
    });

    registry.register(http_client_ctx);

    registry.natives_mut().register(
        "duke/net/HttpClient",
        "fetch",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_http_fetch,
    );
}

#[allow(clippy::needless_pass_by_value)]
fn native_http_fetch(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _output: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let url_ref_opt = match &args[0] {
        Slot::Reference(opt) => *opt,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Ljava/lang/String;",
                got: args[0].type_name(),
            });
        }
    };

    let url_str = if let Some(r) = url_ref_opt {
        if let Some(s) = heap.get(r)?.string_value.as_ref() {
            s.clone()
        } else {
            return Err(VmError::TypeMismatch {
                expected: "String content",
                got: "None",
            });
        }
    } else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NullPointerException".to_string(),
        });
    };

    let without_http = url_str.strip_prefix("http://").unwrap_or(&url_str);
    let (host_port, path) = match without_http.find('/') {
        Some(idx) => (&without_http[..idx], &without_http[idx..]),
        None => (without_http, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let p = host_port[idx + 1..].parse::<u16>().unwrap_or(80);
            (&host_port[..idx], p)
        }
        None => (host_port, 80u16),
    };

    let address = format!("{host}:{port}");
    let mut stream = TcpStream::connect(&address).map_err(|_e| VmError::JavaException {
        class_name: "java/io/IOException".to_string(),
    })?;

    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|_e| VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|_e| VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        })?;

    let body = if let Some(idx) = response.find("\r\n\r\n") {
        &response[idx + 4..]
    } else {
        &response
    };

    let result_ref = heap.allocate_string(body.to_string());
    Ok(Some(Slot::Reference(Some(result_ref))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_stdlib;
    use std::net::TcpListener;

    #[test]
    fn test_native_http_fetch() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Some(mut s) = listener.incoming().flatten().next() {
                let mut buf = [0; 1024];
                let _ = s.read(&mut buf);
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
                let _ = s.write_all(response.as_bytes());
            }
        });

        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        register(&mut registry);

        let url = format!("http://127.0.0.1:{port}/test");
        let url_ref = heap.allocate_string(url);
        let args = vec![Slot::Reference(Some(url_ref))];

        let mut output = Vec::new();
        let result = native_http_fetch(&args, &mut heap, &mut output)
            .unwrap()
            .unwrap();

        if let Slot::Reference(Some(r)) = result {
            let body = heap.get(r).unwrap().string_value.as_ref().unwrap();
            assert_eq!(body, "Hello, World!");
        } else {
            panic!("Expected reference to string");
        }
    }
}
