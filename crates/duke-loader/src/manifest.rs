//! MANIFEST.MF parser — extracts the `Main-Class` attribute from JAR manifests.

/// Parse the `Main-Class` attribute from a `META-INF/MANIFEST.MF` file.
///
/// **Why it exists:** When executing a JAR file via `java -jar`, the JVM needs to
/// know the entry point of the application. This function isolates the logic required
/// to quickly scan the manifest and extract this critical routing information.
///
/// Parses the `Main-Class` attribute from a standard Java `MANIFEST.MF` file.
///
/// The returned class name is automatically converted from the external dot notation
/// (`com.example.App`) to the JVM internal slash notation (`com/example/App`).
/// Returns `None` if the manifest does not declare a main class entry point.
///
/// # Examples
///
/// ```
/// use duke_loader::parse_main_class;
///
/// let manifest = b"Manifest-Version: 1.0\r\nMain-Class: com.example.App\r\n";
/// let main_class = parse_main_class(manifest).unwrap();
/// assert_eq!(main_class, "com/example/App");
/// ```
#[must_use]
pub fn parse_main_class(manifest_bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("Main-Class:") {
            return Some(value.trim().replace('.', "/"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_main_class() {
        let manifest = b"Manifest-Version: 1.0\r\nMain-Class: com.example.App\r\n";
        assert_eq!(
            parse_main_class(manifest),
            Some("com/example/App".to_string())
        );
    }

    #[test]
    fn parses_main_class_unix_line_endings() {
        let manifest = b"Manifest-Version: 1.0\nMain-Class: HelloWorld\n";
        assert_eq!(parse_main_class(manifest), Some("HelloWorld".to_string()));
    }

    #[test]
    fn returns_none_when_missing() {
        let manifest = b"Manifest-Version: 1.0\r\nCreated-By: 21 (OpenJDK)\r\n";
        assert_eq!(parse_main_class(manifest), None);
    }

    #[test]
    fn handles_empty_manifest() {
        assert_eq!(parse_main_class(b""), None);
    }

    #[test]
    fn trims_whitespace() {
        let manifest = b"Main-Class:   com.example.App   \r\n";
        assert_eq!(
            parse_main_class(manifest),
            Some("com/example/App".to_string())
        );
    }

    #[test]
    fn dots_to_slashes() {
        let manifest = b"Main-Class: org.apache.tools.Main\n";
        assert_eq!(
            parse_main_class(manifest),
            Some("org/apache/tools/Main".to_string())
        );
    }
}
