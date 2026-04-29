import sys

def main():
    with open('duke/src/jar_bbcfg.rs', 'r') as f:
        content = f.read()

    content = content.replace(
        "let _ = write!(class_html, \"<title>CFG: {class_name_internal}</title>\\n\");",
        "let _ = writeln!(class_html, \"<title>CFG: {class_name_internal}</title>\");"
    )
    content = content.replace(
        "let _ = write!(class_html, \"<h1>Class: {}</h1>\\n\", class_name_internal.replace('/', \".\"));",
        "let _ = writeln!(class_html, \"<h1>Class: {}</h1>\", class_name_internal.replace('/', \".\"));"
    )
    content = content.replace(
        "let _ = write!(class_html, \"<h2>Method: {name_str}{desc_str}</h2>\\n\");",
        "let _ = writeln!(class_html, \"<h2>Method: {name_str}{desc_str}</h2>\");"
    )
    content = content.replace(
        "let _ = write!(index_html, \"<title>JAR BBCFG Report: {jar_path}</title>\\n\");",
        "let _ = writeln!(index_html, \"<title>JAR BBCFG Report: {jar_path}</title>\");"
    )
    content = content.replace(
        "let _ = write!(index_html, \"  <li><a href=\\\"{link}\\\">{}</a></li>\\n\", name.replace('/', \".\"));",
        "let _ = writeln!(index_html, \"  <li><a href=\\\"{link}\\\">{}</a></li>\", name.replace('/', \".\"));"
    )

    with open('duke/src/jar_bbcfg.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
