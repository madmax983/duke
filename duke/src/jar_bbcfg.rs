#[cfg(feature = "nova")]
use duke_bytecode::{build_basic_blocks, decode, generate_basic_block_cfg};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse,
    types::{AttributeData, ClassFile, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::{fmt::Write, fs, path::Path};

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

/// Generates a static HTML site containing Mermaid Basic Block CFGs for every method in a JAR.
///
/// **Why it exists:** Visualizing control flow across an entire library is difficult.
/// This exporter iterates through a JAR and produces interactive Mermaid diagrams,
/// allowing developers to see the fundamental shape of all bytecode in one place.
#[cfg(feature = "nova")]
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::collapsible_if
)]
pub fn export_jar_bbcfg(jar_path: &str, output_dir: &str) -> std::io::Result<()> {
    let loader = ZipLoader::open(Path::new(jar_path)).map_err(|e| {
        std::io::Error::other(format!("duke: failed to open JAR '{jar_path}': {e}"))
    })?;

    fs::create_dir_all(output_dir)?;

    let reader = loader.reader();
    let mut class_names = Vec::new();

    for entry_name in reader.entry_names() {
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        let Ok(bytes) = loader.find_class(class_name_internal) else {
            continue;
        };
        let Ok(cf) = parse(&bytes) else {
            continue;
        };

        let mut class_html = String::new();
        class_html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        class_html.push_str("<meta charset=\"utf-8\">\n");
        let _ = writeln!(class_html, "<title>CFG: {class_name_internal}</title>");
        class_html.push_str("<script type=\"module\">\n");
        class_html.push_str("  import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';\n");
        class_html.push_str("  mermaid.initialize({ startOnLoad: true });\n");
        class_html.push_str("</script>\n</head>\n<body style=\"font-family: sans-serif;\">\n");
        let _ = writeln!(
            class_html,
            "<h1>Class: {}</h1>",
            class_name_internal.replace('/', ".")
        );
        class_html.push_str("<a href=\"index.html\">Back to Index</a>\n<hr>\n");

        for method in &cf.methods {
            let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
            let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");

            for attr in &method.attributes {
                if let AttributeData::Code(code) = &attr.data {
                    if let Ok(instructions) = decode(&code.code) {
                        let blocks = build_basic_blocks(&instructions);
                        let cfg_mermaid = generate_basic_block_cfg(&blocks);

                        let _ = writeln!(class_html, "<h2>Method: {name_str}{desc_str}</h2>");
                        class_html.push_str("<pre class=\"mermaid\">\n");
                        class_html.push_str(&cfg_mermaid);
                        class_html.push_str("\n</pre>\n<hr>\n");
                    }
                }
            }
        }

        class_html.push_str("</body>\n</html>");

        let safe_name = class_name_internal.replace('/', "_");
        let file_name = format!("{safe_name}.html");
        let out_path = Path::new(output_dir).join(&file_name);

        if fs::write(&out_path, class_html).is_ok() {
            class_names.push((class_name_internal.to_string(), file_name));
        }
    }

    class_names.sort();

    let mut index_html = String::new();
    index_html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    let _ = writeln!(index_html, "<title>JAR BBCFG Report: {jar_path}</title>");
    index_html.push_str("</head>\n<body style=\"font-family: sans-serif;\">\n");
    let _ = write!(
        index_html,
        "<h1>Basic Block CFG Export: {jar_path}</h1>\n<ul>\n"
    );

    for (name, link) in class_names {
        let _ = writeln!(
            index_html,
            "  <li><a href=\"{link}\">{}</a></li>",
            name.replace('/', ".")
        );
    }
    index_html.push_str("</ul>\n</body>\n</html>");

    let index_path = Path::new(output_dir).join("index.html");
    fs::write(&index_path, index_html)?;

    println!("✅ Generated JAR BBCFG site at {output_dir}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "nova")]
    fn test_export_jar_bbcfg() {
        use super::*;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let out_dir = dir.path().to_str().unwrap();

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        export_jar_bbcfg(path_str, out_dir).unwrap();

        let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(index.contains("Basic Block CFG Export:"));
        assert!(index.contains("HelloWorld.html"));
    }
}
