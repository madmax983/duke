use duke_bytecode::{decode, generate_mermaid_cfg};
use duke_classfile::{
    ClassFile,
    types::{AttributeData, CpEntry, CpIndex},
};
use std::fmt::Write;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

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

fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> &str {
    if idx.0 == 0 {
        return "<none>";
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index).unwrap_or("<invalid utf8>")
    } else {
        "<not a class ref>"
    }
}

/// Generates a complete, interactive HTML report of the given class file.
/// Includes constants, fields, methods, and embedded Mermaid control flow graphs.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn generate_html_report(cf: &ClassFile) -> String {
    let mut out = String::new();
    let this_name = escape_html(resolve_class_name(cf, cf.this_class));
    let super_name = escape_html(resolve_class_name(cf, cf.super_class));

    let _ = writeln!(&mut out, "<!DOCTYPE html>");
    let _ = writeln!(&mut out, "<html lang=\"en\">");
    let _ = writeln!(&mut out, "<head>");
    let _ = writeln!(&mut out, "  <meta charset=\"UTF-8\">");
    let _ = writeln!(
        &mut out,
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    );
    let _ = writeln!(
        &mut out,
        "  <title>Duke Class Report: {this_name}</title>"
    );
    let _ = writeln!(&mut out, "  <style>");
    let _ = writeln!(
        &mut out,
        "    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f9f9f9; }}"
    );
    let _ = writeln!(
        &mut out,
        "    h1, h2, h3 {{ color: #2c3e50; border-bottom: 2px solid #eaecef; padding-bottom: 0.3em; }}"
    );
    let _ = writeln!(
        &mut out,
        "    .card {{ background: #fff; border: 1px solid #e1e4e8; border-radius: 6px; padding: 20px; margin-bottom: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.04); }}"
    );
    let _ = writeln!(
        &mut out,
        "    pre {{ background: #f6f8fa; border-radius: 6px; padding: 16px; overflow: auto; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 85%; margin: 0; }}"
    );
    let _ = writeln!(
        &mut out,
        "    .method {{ margin-top: 30px; border-left: 4px solid #0366d6; padding-left: 15px; }}"
    );
    let _ = writeln!(
        &mut out,
        "    .mermaid {{ margin: 20px 0; background: #fff; border: 1px solid #e1e4e8; border-radius: 6px; padding: 20px; display: flex; justify-content: center; }}"
    );
    let _ = writeln!(
        &mut out,
        "    table {{ width: 100%; border-collapse: collapse; margin-top: 10px; }}"
    );
    let _ = writeln!(
        &mut out,
        "    th, td {{ padding: 8px 12px; border: 1px solid #dfe2e5; text-align: left; }}"
    );
    let _ = writeln!(
        &mut out,
        "    th {{ background: #f6f8fa; font-weight: 600; }}"
    );
    let _ = writeln!(
        &mut out,
        "    .badge {{ display: inline-block; padding: 0.25em 0.5em; font-size: 75%; font-weight: 600; line-height: 1; text-align: center; white-space: nowrap; vertical-align: baseline; border-radius: 0.25rem; background-color: #e1e4e8; color: #586069; margin-right: 5px; }}"
    );
    let _ = writeln!(&mut out, "  </style>");
    let _ = writeln!(&mut out, "  <script type=\"module\">");
    let _ = writeln!(
        &mut out,
        "    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';"
    );
    let _ = writeln!(
        &mut out,
        "    mermaid.initialize({{ startOnLoad: true, theme: 'default' }});"
    );
    let _ = writeln!(&mut out, "  </script>");
    let _ = writeln!(&mut out, "</head>");
    let _ = writeln!(&mut out, "<body>");

    // Header
    let _ = writeln!(
        &mut out,
        "  <h1>&#x1F4C4; Class Report: <code>{this_name}</code></h1>"
    );
    let _ = writeln!(&mut out, "  <div class=\"card\">");
    let _ = writeln!(
        &mut out,
        "    <p><strong>Superclass:</strong> <code>{super_name}</code></p>"
    );
    let _ = writeln!(
        &mut out,
        "    <p><strong>Version:</strong> {}.{}</p>",
        cf.major_version, cf.minor_version
    );
    let _ = writeln!(
        &mut out,
        "    <p><strong>Access Flags:</strong> {:?}</p>",
        cf.access_flags
    );
    if !cf.interfaces.is_empty() {
        let _ = writeln!(&mut out, "    <p><strong>Interfaces:</strong></p>");
        let _ = writeln!(&mut out, "    <ul>");
        for idx in &cf.interfaces {
            let intf_name = escape_html(resolve_class_name(cf, *idx));
            let _ = writeln!(
                &mut out,
                "      <li><code>{intf_name}</code></li>"
            );
        }
        let _ = writeln!(&mut out, "    </ul>");
    }
    let _ = writeln!(&mut out, "  </div>");

    // Fields
    if !cf.fields.is_empty() {
        let _ = writeln!(&mut out, "  <h2>Fields</h2>");
        let _ = writeln!(&mut out, "  <div class=\"card\">");
        let _ = writeln!(&mut out, "    <table>");
        let _ = writeln!(
            &mut out,
            "      <thead><tr><th>Access</th><th>Name</th><th>Descriptor</th></tr></thead>"
        );
        let _ = writeln!(&mut out, "      <tbody>");
        for field in &cf.fields {
            let name = escape_html(cp_str(cf, field.name_index).unwrap_or("<invalid>"));
            let desc = escape_html(cp_str(cf, field.descriptor_index).unwrap_or("<invalid>"));
            let _ = writeln!(&mut out, "        <tr>");
            let _ = writeln!(&mut out, "          <td>{:?}</td>", field.access_flags);
            let _ = writeln!(&mut out, "          <td><code>{name}</code></td>");
            let _ = writeln!(&mut out, "          <td><code>{desc}</code></td>");
            let _ = writeln!(&mut out, "        </tr>");
        }
        let _ = writeln!(&mut out, "      </tbody>");
        let _ = writeln!(&mut out, "    </table>");
        let _ = writeln!(&mut out, "  </div>");
    }

    // Methods
    let _ = writeln!(&mut out, "  <h2>Methods</h2>");
    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");

        let ctor_label = if name_str == "<init>" {
            " (Constructor)"
        } else if name_str == "<clinit>" {
            " (Static Initializer)"
        } else {
            ""
        };

        let name = escape_html(name_str);
        let desc = escape_html(desc_str);

        let _ = writeln!(
            &mut out,
            "  <div class=\"method\">"
        );
        let _ = writeln!(
            &mut out,
            "    <h3><code>{name}{desc}{ctor_label}</code></h3>"
        );
        let _ = writeln!(
            &mut out,
            "    <p><span class=\"badge\">{:?}</span></p>",
            method.access_flags
        );

        let mut found_code = false;
        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                found_code = true;
                let _ = writeln!(&mut out, "    <div class=\"card\">");
                let _ = writeln!(
                    &mut out,
                    "      <p><strong>Code Size:</strong> {} bytes | <strong>Max Stack:</strong> {} | <strong>Max Locals:</strong> {}</p>",
                    code.code.len(),
                    code.max_stack,
                    code.max_locals
                );

                match decode(&code.code) {
                    Ok(instructions) => {
                        // Generate Mermaid CFG
                        let cfg = generate_mermaid_cfg(&instructions);
                        let _ = writeln!(&mut out, "      <h4>Control Flow Graph</h4>");
                        let _ = writeln!(&mut out, "      <div class=\"mermaid\">");
                        let _ = writeln!(&mut out, "{cfg}");
                        let _ = writeln!(&mut out, "      </div>");

                        // Also show raw bytecode for reference
                        let _ = writeln!(&mut out, "      <details>");
                        let _ = writeln!(
                            &mut out,
                            "        <summary>View Raw Bytecode Instructions</summary>"
                        );
                        let _ = writeln!(&mut out, "        <pre>");
                        for (pc, instr) in &instructions {
                            let mnemonic = instr.mnemonic();
                            let _ = writeln!(&mut out, "  {pc:4}: {mnemonic}");
                        }
                        let _ = writeln!(&mut out, "        </pre>");
                        let _ = writeln!(&mut out, "      </details>");
                    }
                    Err(e) => {
                        let _ = writeln!(
                            &mut out,
                            "      <p style=\"color: red;\"><strong>Decode Error:</strong> {e}</p>"
                        );
                    }
                }
                let _ = writeln!(&mut out, "    </div>");
            }
        }

        if !found_code {
            let _ = writeln!(
                &mut out,
                "    <p><em>No Code attribute (abstract or native).</em></p>"
            );
        }

        let _ = writeln!(&mut out, "  </div>");
    }

    let _ = writeln!(&mut out, "</body>");
    let _ = writeln!(&mut out, "</html>");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_html_report_empty_class() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![],
            access_flags: duke_classfile::access_flags::ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        let html = generate_html_report(&cf);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Duke Class Report: &lt;none&gt;</title>"));
    }
}
