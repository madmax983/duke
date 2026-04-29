import sys

def main():
    with open('duke/src/jar_bbcfg.rs', 'r') as f:
        content = f.read()

    content = content.replace(
        "class_html.push_str(&format!(\"<title>CFG: {class_name_internal}</title>\\n\"));",
        "let _ = write!(class_html, \"<title>CFG: {class_name_internal}</title>\\n\");"
    )
    content = content.replace(
        "class_html.push_str(&format!(\"<h1>Class: {}</h1>\\n\", class_name_internal.replace('/', \".\")));",
        "let _ = write!(class_html, \"<h1>Class: {}</h1>\\n\", class_name_internal.replace('/', \".\"));"
    )
    content = content.replace(
        "class_html.push_str(&format!(\"<h2>Method: {name_str}{desc_str}</h2>\\n\"));",
        "let _ = write!(class_html, \"<h2>Method: {name_str}{desc_str}</h2>\\n\");"
    )
    content = content.replace(
        "index_html.push_str(&format!(\"<title>JAR BBCFG Report: {jar_path}</title>\\n\"));",
        "let _ = write!(index_html, \"<title>JAR BBCFG Report: {jar_path}</title>\\n\");"
    )
    content = content.replace(
        "index_html.push_str(&format!(\"<h1>Basic Block CFG Export: {jar_path}</h1>\\n<ul>\\n\"));",
        "let _ = write!(index_html, \"<h1>Basic Block CFG Export: {jar_path}</h1>\\n<ul>\\n\");"
    )
    content = content.replace(
        "index_html.push_str(&format!(\"  <li><a href=\\\"{link}\\\">{}</a></li>\\n\", name.replace('/', \".\")));",
        "let _ = write!(index_html, \"  <li><a href=\\\"{link}\\\">{}</a></li>\\n\", name.replace('/', \".\"));"
    )

    if "std::fmt::Write" not in content:
        content = content.replace("use std::{fs, path::Path};", "use std::{fmt::Write, fs, path::Path};")

    with open('duke/src/jar_bbcfg.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
