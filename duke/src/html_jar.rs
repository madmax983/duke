use duke_classfile::parse;
use duke_loader::{ClassLoader, ZipLoader};
use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::html::generate_html_report;

/// Generates a static HTML site for an entire JAR file.
///
/// **Why it exists:** Provides a "`JavaDoc` for Bytecode" experience, generating
/// an interconnected set of HTML reports for every class in a JAR, plus an index.
#[cfg(feature = "nova")]
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[allow(clippy::collapsible_if)]
pub fn generate_jar_html_site(jar_path: &str, output_dir: &str) -> std::io::Result<()> {
    let loader = ZipLoader::open(Path::new(jar_path)).map_err(|e| {
        std::io::Error::other(

            format!("duke: failed to open JAR '{jar_path}': {e}"),
        )
    })?;

    fs::create_dir_all(output_dir)?;

    let mut class_names = Vec::new();
    let reader = loader.reader();
    let entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let html = generate_html_report(&cf);

                let safe_name = class_name_internal.replace('/', "_");
                let file_name = format!("{safe_name}.html");
                let out_path = Path::new(output_dir).join(&file_name);

                if fs::write(&out_path, html).is_ok() {
                    class_names.push((class_name_internal.to_string(), file_name));
                }
            }
        }
    }

    class_names.sort();

    let mut index_html = String::new();
    let _ = writeln!(&mut index_html, "<!DOCTYPE html>");
    let _ = writeln!(&mut index_html, "<html>");
    let _ = writeln!(&mut index_html, "<head>");
    let _ = writeln!(&mut index_html, "  <meta charset=\"utf-8\">");
    let _ = writeln!(&mut index_html, "  <title>JAR Report: {jar_path}</title>");
    let _ = writeln!(&mut index_html, "  <style>");
    let _ = writeln!(
        &mut index_html,
        "    body {{ font-family: sans-serif; margin: 2rem; background: #f9fafb; color: #111827; }}"
    );
    let _ = writeln!(
        &mut index_html,
        "    h1 {{ color: #1f2937; border-bottom: 2px solid #e5e7eb; padding-bottom: 0.5rem; }}"
    );
    let _ = writeln!(
        &mut index_html,
        "    ul {{ list-style-type: none; padding: 0; }}"
    );
    let _ = writeln!(&mut index_html, "    li {{ margin-bottom: 0.5rem; }}");
    let _ = writeln!(
        &mut index_html,
        "    a {{ color: #2563eb; text-decoration: none; font-family: monospace; font-size: 1.1em; }}"
    );
    let _ = writeln!(
        &mut index_html,
        "    a:hover {{ text-decoration: underline; }}"
    );
    let _ = writeln!(&mut index_html, "  </style>");
    let _ = writeln!(&mut index_html, "</head>");
    let _ = writeln!(&mut index_html, "<body>");
    let _ = writeln!(&mut index_html, "  <h1>JAR Analysis Report</h1>");
    let _ = writeln!(
        &mut index_html,
        "  <p><strong>Source:</strong> <code>{jar_path}</code></p>"
    );
    let _ = writeln!(
        &mut index_html,
        "  <p><strong>Classes:</strong> {}</p>",
        class_names.len()
    );
    let _ = writeln!(&mut index_html, "  <ul>");
    for (name, link) in &class_names {
        let display_name = name.replace('/', ".");
        let _ = writeln!(
            &mut index_html,
            "    <li><a href=\"{link}\">{display_name}</a></li>"
        );
    }
    let _ = writeln!(&mut index_html, "  </ul>");
    let _ = writeln!(&mut index_html, "</body>");
    let _ = writeln!(&mut index_html, "</html>");

    let index_path = Path::new(output_dir).join("index.html");
    fs::write(&index_path, index_html)?;

    println!("✅ Generated static HTML site at {output_dir}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[cfg(feature = "nova")]
    fn test_generate_jar_html_site() {
        let dir = tempdir().unwrap();
        let out_dir = dir.path().to_str().unwrap();

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        generate_jar_html_site(path_str, out_dir).unwrap();

        let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(index.contains("JAR Analysis Report"));
        assert!(index.contains("HelloWorld.html"));
    }
}
