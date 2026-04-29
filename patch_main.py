import sys

def main():
    with open('duke/src/main.rs', 'r') as f:
        lines = f.readlines()

    out = []
    for line in lines:
        if line.strip() == "mod html_jar;":
            out.append(line)
            out.append("#[cfg(feature = \"nova\")]\n")
            out.append("mod jar_bbcfg;\n")
        elif line.strip() == "eprintln!(\"       duke html-jar <file.jar> <output_dir>\");":
            out.append(line)
            out.append("        #[cfg(feature = \"nova\")]\n")
            out.append("        eprintln!(\"       duke bbcfg-jar <file.jar> <output_dir>\");\n")
        elif line.strip() == "if args.len() >= 4 && args[1] == \"html-jar\" {":
            out.append("    #[cfg(feature = \"nova\")]\n")
            out.append("    if args.len() >= 4 && args[1] == \"bbcfg-jar\" {\n")
            out.append("        if let Err(e) = jar_bbcfg::export_jar_bbcfg(&args[2], &args[3]) {\n")
            out.append("            eprintln!(\"{e}\");\n")
            out.append("            process::exit(1);\n")
            out.append("        }\n")
            out.append("        return;\n")
            out.append("    }\n")
            out.append("\n")
            out.append(line)
        else:
            out.append(line)

    with open('duke/src/main.rs', 'w') as f:
        f.writelines(out)

if __name__ == '__main__':
    main()
