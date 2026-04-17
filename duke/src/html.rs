use duke_bytecode::{build_basic_blocks, decode, generate_basic_block_cfg, generate_mermaid_cfg};
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

fn write_html_header(out: &mut String, title: &str) {
    let _ = writeln!(out, "<!DOCTYPE html>");
    let _ = writeln!(out, "<html lang=\"en\">");
    let _ = writeln!(out, "<head>");
    let _ = writeln!(out, "  <meta charset=\"UTF-8\">");
    let _ = writeln!(
        out,
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    );
    let _ = writeln!(out, "  <title>Duke Class Report: {title}</title>");
    let _ = writeln!(out, "  <style>");
    let _ = writeln!(
        out,
        "    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f9f9f9; }}"
    );
    let _ = writeln!(
        out,
        "    h1, h2, h3 {{ color: #2c3e50; border-bottom: 2px solid #eaecef; padding-bottom: 0.3em; }}"
    );
    let _ = writeln!(
        out,
        "    .card {{ background: #fff; border: 1px solid #e1e4e8; border-radius: 6px; padding: 20px; margin-bottom: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.04); }}"
    );
    let _ = writeln!(
        out,
        "    pre {{ background: #f6f8fa; border-radius: 6px; padding: 16px; overflow: auto; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 85%; margin: 0; }}"
    );
    let _ = writeln!(
        out,
        "    .method {{ margin-top: 30px; border-left: 4px solid #0366d6; padding-left: 15px; }}"
    );
    let _ = writeln!(
        out,
        "    .mermaid {{ margin: 20px 0; background: #fff; border: 1px solid #e1e4e8; border-radius: 6px; padding: 20px; display: flex; justify-content: center; }}"
    );
    let _ = writeln!(
        out,
        "    table {{ width: 100%; border-collapse: collapse; margin-top: 10px; }}"
    );
    let _ = writeln!(
        out,
        "    th, td {{ padding: 8px 12px; border: 1px solid #dfe2e5; text-align: left; }}"
    );
    let _ = writeln!(out, "    th {{ background: #f6f8fa; font-weight: 600; }}");
    let _ = writeln!(
        out,
        "    .badge {{ display: inline-block; padding: 0.25em 0.5em; font-size: 75%; font-weight: 600; line-height: 1; text-align: center; white-space: nowrap; vertical-align: baseline; border-radius: 0.25rem; background-color: #e1e4e8; color: #586069; margin-right: 5px; }}"
    );
    let _ = writeln!(out, "  </style>");
    let _ = writeln!(out, "  <script type=\"module\">");
    let _ = writeln!(
        out,
        "    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';"
    );
    let _ = writeln!(
        out,
        "    mermaid.initialize({{ startOnLoad: true, theme: 'default' }});"
    );
    let _ = writeln!(out, "  </script>");
    let _ = writeln!(out, "</head>");
    let _ = writeln!(out, "<body>");
}

fn write_class_summary(out: &mut String, cf: &ClassFile) {
    let this_name = escape_html(resolve_class_name(cf, cf.this_class));
    let super_name = escape_html(resolve_class_name(cf, cf.super_class));

    let _ = writeln!(
        out,
        "  <h1>&#x1F4C4; Class Report: <code>{this_name}</code></h1>"
    );
    let _ = writeln!(out, "  <div class=\"card\">");
    let _ = writeln!(
        out,
        "    <p><strong>Superclass:</strong> <code>{super_name}</code></p>"
    );
    let _ = writeln!(
        out,
        "    <p><strong>Version:</strong> {}.{}</p>",
        cf.major_version, cf.minor_version
    );
    let _ = writeln!(
        out,
        "    <p><strong>Access Flags:</strong> {:?}</p>",
        cf.access_flags
    );
    if !cf.interfaces.is_empty() {
        let _ = writeln!(out, "    <p><strong>Interfaces:</strong></p>");
        let _ = writeln!(out, "    <ul>");
        for idx in &cf.interfaces {
            let intf_name = escape_html(resolve_class_name(cf, *idx));
            let _ = writeln!(out, "      <li><code>{intf_name}</code></li>");
        }
        let _ = writeln!(out, "    </ul>");
    }
    let _ = writeln!(out, "  </div>");
}

fn write_fields(out: &mut String, cf: &ClassFile) {
    if cf.fields.is_empty() {
        return;
    }

    let _ = writeln!(out, "  <h2>Fields</h2>");
    let _ = writeln!(out, "  <div class=\"card\">");
    let _ = writeln!(out, "    <table>");
    let _ = writeln!(
        out,
        "      <thead><tr><th>Access</th><th>Name</th><th>Descriptor</th></tr></thead>"
    );
    let _ = writeln!(out, "      <tbody>");
    for field in &cf.fields {
        let name = escape_html(cp_str(cf, field.name_index).unwrap_or("<invalid>"));
        let desc = escape_html(cp_str(cf, field.descriptor_index).unwrap_or("<invalid>"));
        let _ = writeln!(out, "        <tr>");
        let _ = writeln!(out, "          <td>{:?}</td>", field.access_flags);
        let _ = writeln!(out, "          <td><code>{name}</code></td>");
        let _ = writeln!(out, "          <td><code>{desc}</code></td>");
        let _ = writeln!(out, "        </tr>");
    }
    let _ = writeln!(out, "      </tbody>");
    let _ = writeln!(out, "    </table>");
    let _ = writeln!(out, "  </div>");
}

fn write_methods(out: &mut String, cf: &ClassFile) {
    let _ = writeln!(out, "  <h2>Methods</h2>");
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

        let _ = writeln!(out, "  <div class=\"method\">");
        let _ = writeln!(out, "    <h3><code>{name}{desc}{ctor_label}</code></h3>");
        let _ = writeln!(
            out,
            "    <p><span class=\"badge\">{:?}</span></p>",
            method.access_flags
        );

        let mut found_code = false;
        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                found_code = true;
                let _ = writeln!(out, "    <div class=\"card\">");
                let _ = writeln!(
                    out,
                    "      <p><strong>Code Size:</strong> {} bytes | <strong>Max Stack:</strong> {} | <strong>Max Locals:</strong> {}</p>",
                    code.code.len(),
                    code.max_stack,
                    code.max_locals
                );

                match decode(&code.code) {
                    Ok(instructions) => {
                        // Generate Mermaid CFG
                        let cfg = generate_mermaid_cfg(&instructions);
                        let _ = writeln!(out, "      <h4>Control Flow Graph</h4>");
                        let _ = writeln!(out, "      <div class=\"mermaid\">");
                        let _ = writeln!(out, "{cfg}");
                        let _ = writeln!(out, "      </div>");

                        // Generate Basic Block CFG
                        let blocks = build_basic_blocks(&instructions);
                        let bbcfg = generate_basic_block_cfg(&blocks);
                        let _ = writeln!(out, "      <h4>Basic Block Control Flow Graph</h4>");
                        let _ = writeln!(out, "      <div class=\"mermaid\">");
                        let _ = writeln!(out, "{bbcfg}");
                        let _ = writeln!(out, "      </div>");

                        // Also show raw bytecode for reference
                        let _ = writeln!(out, "      <details>");
                        let _ = writeln!(
                            out,
                            "        <summary>View Raw Bytecode Instructions</summary>"
                        );
                        let _ = writeln!(out, "        <pre>");
                        for (pc, instr) in &instructions {
                            let mnemonic = instr.mnemonic();
                            let _ = writeln!(out, "  {pc:4}: {mnemonic}");
                        }
                        let _ = writeln!(out, "        </pre>");
                        let _ = writeln!(out, "      </details>");
                    }
                    Err(e) => {
                        let _ = writeln!(
                            out,
                            "      <p style=\"color: red;\"><strong>Decode Error:</strong> {e}</p>"
                        );
                    }
                }
                let _ = writeln!(out, "    </div>");
            }
        }

        if !found_code {
            let _ = writeln!(
                out,
                "    <p><em>No Code attribute (abstract or native).</em></p>"
            );
        }

        let _ = writeln!(out, "  </div>");
    }
}

/// Generates a complete, interactive HTML report of the given class file.
///
/// **Why it exists:** Console outputs and raw JSON dumps are hard to navigate.
/// This generates a beautifully formatted, interactive HTML document that allows
/// developers to visually explore the class constants, fields, and even see
/// embedded Mermaid control flow graphs for every method in their browser.
///
/// Includes constants, fields, methods, and embedded Mermaid control flow graphs.
///
/// # Examples
///
/// ```ignore
/// use duke::html::generate_html_report;
/// use duke_classfile::ClassFile;
///
/// let cf: ClassFile = get_parsed_class_somehow();
/// let html_string = generate_html_report(&cf);
/// std::fs::write("report.html", html_string).unwrap();
/// ```
#[must_use]
pub fn generate_html_report(cf: &ClassFile) -> String {
    let mut out = String::new();
    let this_name = escape_html(resolve_class_name(cf, cf.this_class));

    write_html_header(&mut out, &this_name);
    write_class_summary(&mut out, cf);
    write_fields(&mut out, cf);
    write_methods(&mut out, cf);

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

    #[test]
    fn test_generate_html_report_complex_class() {
        use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};
        use duke_classfile::types::{
            AttributeData, AttributeInfo, CodeAttribute, FieldInfo, MethodInfo,
        };

        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0 is reserved
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }), // 1
                Some(CpEntry::Utf8("java/lang/Object".to_string())), // 2
                Some(CpEntry::Class {
                    name_index: CpIndex(4),
                }), // 3
                Some(CpEntry::Utf8("MyClass<>&".to_string())), // 4
                Some(CpEntry::Class {
                    name_index: CpIndex(6),
                }), // 5
                Some(CpEntry::Utf8("java/lang/Runnable".to_string())), // 6
                Some(CpEntry::Utf8("myField".to_string())), // 7
                Some(CpEntry::Utf8("I".to_string())), // 8
                Some(CpEntry::Utf8("myMethod".to_string())), // 9
                Some(CpEntry::Utf8("()V".to_string())), // 10
                Some(CpEntry::Utf8("<init>".to_string())), // 11
                Some(CpEntry::Utf8("<clinit>".to_string())), // 12
                Some(CpEntry::Utf8("run".to_string())), // 13
            ],
            access_flags: duke_classfile::access_flags::ClassAccessFlags::PUBLIC,
            this_class: CpIndex(3),
            super_class: CpIndex(1),
            interfaces: vec![CpIndex(5)],
            fields: vec![FieldInfo {
                access_flags: FieldAccessFlags::PRIVATE,
                name_index: CpIndex(7),
                descriptor_index: CpIndex(8),
                attributes: vec![],
            }],
            methods: vec![
                MethodInfo {
                    access_flags: MethodAccessFlags::PUBLIC,
                    name_index: CpIndex(9),
                    descriptor_index: CpIndex(10),
                    attributes: vec![AttributeInfo {
                        name_index: CpIndex(0), // Dummy
                        data: AttributeData::Code(CodeAttribute {
                            max_stack: 1,
                            max_locals: 1,
                            code: vec![0xb1], // return
                            exception_table: vec![],
                            attributes: vec![],
                        }),
                    }],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::PUBLIC,
                    name_index: CpIndex(11), // <init>
                    descriptor_index: CpIndex(10),
                    attributes: vec![],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::STATIC,
                    name_index: CpIndex(12), // <clinit>
                    descriptor_index: CpIndex(10),
                    attributes: vec![],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::PUBLIC,
                    name_index: CpIndex(13), // run (invalid code to trigger decode error)
                    descriptor_index: CpIndex(10),
                    attributes: vec![AttributeInfo {
                        name_index: CpIndex(0),
                        data: AttributeData::Code(CodeAttribute {
                            max_stack: 1,
                            max_locals: 1,
                            code: vec![0xfe], // Invalid opcode IMPDEP1
                            exception_table: vec![],
                            attributes: vec![],
                        }),
                    }],
                },
            ],
            attributes: vec![],
        };
        let html = generate_html_report(&cf);

        // Assertions for escape_html and resolve_class_name
        assert!(html.contains("Duke Class Report: MyClass&lt;&gt;&amp;"));
        assert!(html.contains("<code>java/lang/Object</code>"));
        assert!(html.contains("<code>java/lang/Runnable</code>"));

        // Assertions for fields
        assert!(html.contains("<td><code>myField</code></td>"));

        // Assertions for methods
        assert!(html.contains("<h3><code>myMethod()V</code></h3>"));
        assert!(html.contains("<h3><code>&lt;init&gt;()V (Constructor)</code></h3>"));
        assert!(html.contains("<h3><code>&lt;clinit&gt;()V (Static Initializer)</code></h3>"));

        // Assertions for decode success/error
        assert!(html.contains("return")); // The mnemonic for 0xb1
        assert!(html.contains("Decode Error:")); // For the invalid opcode
        assert!(html.contains("<h4>Control Flow Graph</h4>"));
        assert!(html.contains("<h4>Basic Block Control Flow Graph</h4>"));
    }
}
