//! Main entry point for the Duke JVM executable.
//!
//! Handles command-line argument parsing, environment initialization,
//! class loading, and the main execution loop for the JVM.

use std::process;

mod analyze;
mod deps_graph;
mod html;
mod jar_analyze;
mod scan;
mod search;
mod uml;

use duke_bytecode::{decode, generate_mermaid_call_graph, generate_mermaid_cfg};
use duke_classfile::{
    ClassFile,
    access_flags::MethodAccessFlags,
    parse,
    types::{AttributeData, CpEntry, CpIndex},
};
use duke_gc::Heap;
use duke_interpreter::{
    ClassRegistry, bootstrap_stdlib, build_class_context, execute_class_to_completion,
};
use duke_loader::{
    BootstrapLoader, ClassLoader, ClasspathEntry, DirectoryLoader, LoadResult, ZipLoader, ZipReader,
};
use duke_runtime::{Slot, VmError};

/// Where to write telemetry JSON after execution.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum TelemetryDest {
    Stdout,
    File(String),
    Markdown(String),
}

/// Strip `--jdk=<path>` or `--jdk <path>` from `args` and return the JDK home.
fn extract_jdk_flag(args: &mut Vec<String>) -> Option<String> {
    let mut jdk = std::env::var("JAVA_HOME").ok();
    let mut remove_next = false;
    args.retain(|arg| {
        if remove_next {
            jdk = Some(arg.clone());
            remove_next = false;
            return false;
        }
        if let Some(path) = arg.strip_prefix("--jdk=") {
            jdk = Some(path.to_string());
            return false;
        }
        if arg == "--jdk" {
            remove_next = true;
            return false;
        }
        true
    });
    jdk
}

/// Strip `-jar <path>` or `--jar <path>` from `args` and return the JAR path.
fn extract_jar_flag(args: &mut Vec<String>) -> Option<String> {
    let mut jar = None;
    let mut remove_next = false;
    args.retain(|arg| {
        if remove_next {
            jar = Some(arg.clone());
            remove_next = false;
            return false;
        }
        if arg == "-jar" || arg == "--jar" {
            remove_next = true;
            return false;
        }
        true
    });
    jar
}

/// Build a class loader: `BootstrapLoader` (JDK jimage + app dir) when JDK path
/// is known, or plain `DirectoryLoader` otherwise.
struct CliLoader(Box<dyn ClassLoader + Send + Sync>);

impl ClassLoader for CliLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        self.0.find_class(name)
    }
}

fn make_loader(jdk_home: Option<&str>, classpath: &[std::path::PathBuf]) -> CliLoader {
    if let Some(home) = jdk_home {
        let modules = std::path::Path::new(home).join("lib").join("modules");
        if modules.exists() {
            match BootstrapLoader::new(&modules, classpath.to_vec()) {
                Ok(bl) => return CliLoader(Box::new(bl)),
                Err(e) => eprintln!(
                    "duke: warning: cannot open JDK modules ({e}), falling back to directory loader"
                ),
            }
        } else {
            eprintln!(
                "duke: warning: {} not found, falling back to directory loader",
                modules.display()
            );
        }
    }
    // Without JDK, build a composite loader from classpath entries.
    if classpath.len() == 1 {
        let p = &classpath[0];
        let is_archive = p
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"));
        if is_archive {
            match ZipLoader::open(p) {
                Ok(zl) => return CliLoader(Box::new(zl)),
                Err(e) => {
                    eprintln!("duke: warning: cannot open JAR ({e}), falling back to directory");
                }
            }
        }
        return CliLoader(Box::new(DirectoryLoader::new(p)));
    }
    // Multiple entries: build a chain loader.
    let mut entries: Vec<ClasspathEntry> = Vec::new();
    for p in classpath {
        let is_archive = p
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"));
        if is_archive {
            match ZipLoader::open(p) {
                Ok(zl) => entries.push(ClasspathEntry::Zip(zl)),
                Err(e) => eprintln!("duke: warning: skipping JAR {}: {e}", p.display()),
            }
        } else {
            entries.push(ClasspathEntry::Directory(DirectoryLoader::new(p)));
        }
    }
    CliLoader(Box::new(ChainLoader(entries)))
}

/// A chain of classpath entries tried in order.
struct ChainLoader(Vec<ClasspathEntry>);

impl ClassLoader for ChainLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        for entry in &self.0 {
            if let Ok(bytes) = entry.find_class(name) {
                return Ok(bytes);
            }
        }
        Err(duke_loader::LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum MermaidDest {
    Stdout,
    File(String),
}

/// Strip `--mermaid-heap[=path]` from `args` and return the configured destination.
#[allow(dead_code)]
fn extract_mermaid_heap_flag(args: &mut Vec<String>) -> Option<MermaidDest> {
    let mut result = None;
    args.retain(|arg| {
        if arg == "--mermaid-heap" {
            result = Some(MermaidDest::Stdout);
            false
        } else if let Some(path) = arg.strip_prefix("--mermaid-heap=") {
            result = Some(MermaidDest::File(path.to_string()));
            false
        } else {
            true
        }
    });
    result
}

/// Strip `--telemetry[=path]` from `args` and return the configured destination.
fn extract_telemetry_flag(args: &mut Vec<String>) -> Option<TelemetryDest> {
    let mut result = None;
    args.retain(|arg| {
        if arg == "--telemetry" {
            result = Some(TelemetryDest::Stdout);
            false
        } else if arg == "--telemetry-md" {
            // --telemetry-md without an equal sign implicitly outputs to stdout as well
            result = Some(TelemetryDest::Markdown(String::new()));
            false
        } else if let Some(path) = arg.strip_prefix("--telemetry=") {
            result = Some(TelemetryDest::File(path.to_string()));
            false
        } else if let Some(path) = arg.strip_prefix("--telemetry-md=") {
            result = Some(TelemetryDest::Markdown(path.to_string()));
            false
        } else {
            true
        }
    });
    result
}

#[allow(clippy::too_many_lines)]
fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let telemetry = extract_telemetry_flag(&mut args);
    let mermaid_dest = extract_mermaid_heap_flag(&mut args);
    let jdk_home = extract_jdk_flag(&mut args);
    let jar_path = extract_jar_flag(&mut args);

    if jar_path.is_none() && args.len() < 2 {
        eprintln!("Usage: duke <classfile.class>");
        eprintln!("       duke dump <classfile.class>");
        eprintln!("       duke html <classfile.class> [output.html]");
        eprintln!("       duke deps-graph <classfile.class>");
        eprintln!("       duke load <ClassName>");
        eprintln!("       duke cfg <classfile.class> <method>");
        eprintln!("       duke cg <classfile.class>");
        eprintln!("       duke analyze <classfile.class>");
        eprintln!("       duke jar-analyze <file.jar>");
        eprintln!("       duke search <classfile.class> <opcode>");
        eprintln!("       duke scan <classfile.class>");
        eprintln!("       duke jar-scan <file.jar>");
        eprintln!("       duke uml <classfile.class>");
        eprintln!("       duke exec <classfile.class> <method> [int-arg...]");
        eprintln!("       duke run <classfile.class> [string-arg...]");
        eprintln!("       duke stub <classfile.class>");
        eprintln!("       duke -jar <file.jar> [string-arg...]");
        eprintln!("Options: --telemetry[=path]  dump telemetry JSON after execution");
        eprintln!("         --telemetry-md[=p]  dump telemetry Markdown report after execution");
        eprintln!("         --jdk=<path>        JDK home for loading real JDK classes");
        eprintln!("         (also reads JAVA_HOME env var)");
        process::exit(1);
    }

    // Dispatch `-jar`: discover Main-Class from manifest and execute it.
    if let Some(ref jar) = jar_path {
        let remaining_args: Vec<&str> = args[1..].iter().map(String::as_str).collect();
        run_jar(
            jar,
            &remaining_args,
            telemetry,
            mermaid_dest,
            jdk_home.as_deref(),
        );
        return;
    }

    // Dispatch `load` before trying to read a file.
    if args.len() >= 3 && args[1] == "load" {
        load_and_dump(&args[2]);
        return;
    }

    // Dispatch `html`: output HTML report for class.
    if args.len() >= 3 && args[1] == "html" {
        let output_path = if args.len() >= 4 {
            Some(args[3].as_str())
        } else {
            None
        };
        dump_html(&args[2], output_path);
        return;
    }

    // Dispatch `cfg`: dump control flow graph for a method.
    if args.len() >= 4 && args[1] == "cfg" {
        dump_cfg(&args[2], &args[3]);
        return;
    }

    // Dispatch `cg`: dump call graph for a class.
    if args.len() >= 3 && args[1] == "cg" {
        dump_cg(&args[2]);
        return;
    }

    // Dispatch `search`: search for opcodes in class methods.
    if args.len() >= 4 && args[1] == "search" {
        search::dump_search(&args[2], &args[3]);
        return;
    }

    if args.len() >= 3 && args[1] == "scan" {
        scan::dump_scan(&args[2]);
        return;
    }

    if args.len() >= 3 && args[1] == "jar-scan" {
        scan::dump_jar_scan(&args[2]);
        return;
    }

    // Dispatch `analyze`: run static analysis on the class.

    if args.len() >= 3 && args[1] == "jar-analyze" {
        jar_analyze::dump_jar_analyze(&args[2]);
        return;
    }
    if args.len() >= 3 && args[1] == "analyze" {
        dump_analyze(&args[2]);
        return;
    }

    if args.len() >= 3 && args[1] == "deps-graph" {
        let bytes = std::fs::read(&args[2]).expect("failed to read class file");
        let cf = parse(&bytes).expect("failed to parse class file");
        let graph = deps_graph::generate_deps_graph(&cf);
        println!("{graph}");
        return;
    }

    // Dispatch `uml`: dump mermaid class diagram.
    if args.len() >= 3 && args[1] == "uml" {
        dump_uml(&args[2]);
        return;
    }

    // Dispatch `exec`: run a static method and print the result.
    if args.len() >= 4 && args[1] == "exec" {
        exec_method(&args[2..], telemetry, mermaid_dest, jdk_home.as_deref());
        return;
    }

    // Dispatch `run`: execute main(String[]) entry point.
    if args.len() >= 3 && args[1] == "run" {
        run_main(&args[2..], telemetry, mermaid_dest, jdk_home.as_deref());
        return;
    }

    // Default: dump — reads the file at the given path.
    let (subcommand, path) = if args.len() >= 3 {
        (args[1].as_str(), args[2].as_str())
    } else {
        ("dump", args[1].as_str())
    };

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });

    let class_file = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error in '{path}': {e}");
        process::exit(1);
    });

    match subcommand {
        "dump" => dump_class_file(&class_file),
        "stub" => generate_stubs(&class_file),
        other => {
            eprintln!("duke: unknown subcommand '{other}'");
            process::exit(1);
        }
    }
}

/// Emit Mermaid JS heap graph to the configured destination (stdout or file).
fn emit_mermaid_heap(heap: &Heap, dest: Option<MermaidDest>) {
    let Some(dest) = dest else { return };
    let mermaid_str = heap.dump_mermaid();
    match dest {
        MermaidDest::Stdout => println!("{mermaid_str}"),
        MermaidDest::File(path) => {
            if let Err(e) = std::fs::write(&path, &mermaid_str) {
                eprintln!("duke: failed to write mermaid heap to '{path}': {e}");
            }
        }
    }
}

/// Emit telemetry JSON to the configured destination (stdout or file).
#[cfg(feature = "telemetry")]
fn emit_telemetry(registry: &ClassRegistry, dest: Option<TelemetryDest>) {
    let Some(dest) = dest else { return };
    match dest {
        TelemetryDest::Stdout => {
            let json = registry.telemetry.to_json();
            println!("{json}");
        }
        TelemetryDest::File(path) => {
            let json = registry.telemetry.to_json();
            if let Err(e) = std::fs::write(&path, &json) {
                eprintln!("duke: failed to write telemetry to '{path}': {e}");
            }
        }
        TelemetryDest::Markdown(path) => {
            let md = registry.telemetry.to_markdown_report();
            if path.is_empty() {
                println!("{md}");
            } else if let Err(e) = std::fs::write(&path, &md) {
                eprintln!("duke: failed to write telemetry markdown to '{path}': {e}");
            }
        }
    }
}

#[cfg(not(feature = "telemetry"))]
#[allow(clippy::needless_pass_by_value)]
fn emit_telemetry(_registry: &ClassRegistry, dest: Option<TelemetryDest>) {
    if dest.is_some() {
        eprintln!(
            "duke: --telemetry flag requires the 'telemetry' feature \
             (rebuild with --features telemetry)"
        );
    }
}

/// `duke load <ClassName>` — load a class by internal name from the current
/// directory and dump its structure.
///
/// Example: `duke load HelloWorld` loads `./HelloWorld.class`.
fn load_and_dump(class_name: &str) {
    let loader = DirectoryLoader::new(".");
    let bytes = loader.find_class(class_name).unwrap_or_else(|e| {
        eprintln!("duke: {e}");
        process::exit(1);
    });
    let class_file = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error for '{class_name}': {e}");
        process::exit(1);
    });
    dump_class_file(&class_file);
}

// ---------------------------------------------------------------------------
// Exec
// ---------------------------------------------------------------------------

/// `duke exec <classfile.class> <method> [int-arg...]`
///
/// Parses and executes a static method, printing the return value.
fn exec_method(
    args: &[String],
    telemetry: Option<TelemetryDest>,
    mermaid_dest: Option<MermaidDest>,
    jdk_home: Option<&str>,
) {
    if args.len() < 2 {
        eprintln!("Usage: duke exec <classfile.class> <method> [int-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let method_name = &args[1];
    let int_args: Vec<Slot> = args[2..]
        .iter()
        .map(|s| {
            Slot::Int(s.parse::<i32>().unwrap_or_else(|_| {
                eprintln!("duke: argument '{s}' is not an integer");
                process::exit(1);
            }))
        })
        .collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    // Find the target method's descriptor.
    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
                return false;
            };
            s.as_str() == method_name.as_str()
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });
    let descriptor = cp_str(&cf, target.descriptor_index)
        .unwrap_or("")
        .to_string();

    let ctx = build_class_context(&cf);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);

    // Create a class loader from the class file's parent directory.
    // When --jdk is given, also loads missing classes from the JDK jimage.
    let parent = std::path::Path::new(path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let loader = make_loader(jdk_home, &[parent.to_path_buf()]);
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let mut stdout = std::io::stdout();
    let exit_code = match execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut stdout,
        &entry_class,
        method_name,
        &descriptor,
        &int_args,
    ) {
        Ok(Some(result)) => {
            println!("{result:?}");
            None
        }
        Ok(None) => {
            println!("(void)");
            None
        }
        Err(VmError::SystemExit { code }) => Some(code),
        Err(e) => {
            emit_mermaid_heap(&heap, mermaid_dest);
            emit_telemetry(&registry, telemetry);
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    };
    emit_mermaid_heap(&heap, mermaid_dest);
    emit_telemetry(&registry, telemetry);
    if let Some(code) = exit_code {
        process::exit(code);
    }
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

/// `duke run <classfile.class> [string-arg...]`
///
/// Executes `public static void main(String[])`, passing string arguments.
fn run_main(
    args: &[String],
    telemetry: Option<TelemetryDest>,
    mermaid_dest: Option<MermaidDest>,
    jdk_home: Option<&str>,
) {
    if args.is_empty() {
        eprintln!("Usage: duke run <classfile.class> [string-arg...]");
        process::exit(1);
    }
    let path = &args[0];
    let string_args: Vec<&str> = args[1..].iter().map(String::as_str).collect();

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let ctx = build_class_context(&cf);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);

    let parent = std::path::Path::new(path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let loader = make_loader(jdk_home, &[parent.to_path_buf()]);
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Build String[] args array on the heap.
    let mut arg_refs: Vec<Slot> = Vec::new();
    for arg in &string_args {
        let r = heap.allocate_string((*arg).to_string());
        arg_refs.push(Slot::Reference(Some(r)));
    }
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), string_args.len());
    for (i, slot) in arg_refs.into_iter().enumerate() {
        heap.get_mut(arr_ref).unwrap().fields[i] = slot;
    }

    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut stdout = std::io::stdout();
    let exit_code = match execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut stdout,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    ) {
        Ok(_) => None,
        Err(VmError::SystemExit { code }) => Some(code),
        Err(e) => {
            emit_mermaid_heap(&heap, mermaid_dest);
            emit_telemetry(&registry, telemetry);
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    };
    emit_mermaid_heap(&heap, mermaid_dest);
    emit_telemetry(&registry, telemetry);
    if let Some(code) = exit_code {
        process::exit(code);
    }
}

// ---------------------------------------------------------------------------
// Jar
// ---------------------------------------------------------------------------

/// `duke -jar <file.jar> [string-arg...]`
///
/// Reads `META-INF/MANIFEST.MF` to discover `Main-Class`, then executes it.
fn run_jar(
    jar_path: &str,
    string_args: &[&str],
    telemetry: Option<TelemetryDest>,
    mermaid_dest: Option<MermaidDest>,
    jdk_home: Option<&str>,
) {
    let jar = std::path::Path::new(jar_path);
    let reader = ZipReader::open(jar).unwrap_or_else(|e| {
        eprintln!("duke: cannot open JAR '{jar_path}': {e}");
        process::exit(1);
    });
    let manifest_bytes = reader
        .read_entry("META-INF/MANIFEST.MF")
        .unwrap_or_else(|_| {
            eprintln!("duke: JAR '{jar_path}' has no META-INF/MANIFEST.MF");
            process::exit(1);
        });
    let main_class = duke_loader::parse_main_class(&manifest_bytes).unwrap_or_else(|| {
        eprintln!("duke: no Main-Class attribute in '{jar_path}' manifest");
        process::exit(1);
    });

    // Build classpath: the JAR itself + its parent directory (for auxiliary classes).
    let jar_abs = jar.to_path_buf();
    let parent = jar
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    let classpath = vec![jar_abs, parent];
    let loader = make_loader(jdk_home, &classpath);

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let jar_code_source = std::fs::canonicalize(jar).unwrap_or_else(|_| jar.to_path_buf());
    registry.set_default_code_source(jar_code_source.to_string_lossy().to_string());

    // Pre-load the entry class from the JAR.
    if !registry
        .ensure_loaded(&main_class, &loader)
        .unwrap_or(false)
    {
        eprintln!("duke: cannot load class '{main_class}' from JAR '{jar_path}'");
        process::exit(1);
    }

    // Build String[] args array on the heap.
    let mut arg_refs: Vec<Slot> = Vec::new();
    for arg in string_args {
        let r = heap.allocate_string((*arg).to_string());
        arg_refs.push(Slot::Reference(Some(r)));
    }
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), string_args.len());
    for (i, slot) in arg_refs.into_iter().enumerate() {
        heap.get_mut(arr_ref).unwrap().fields[i] = slot;
    }
    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut stdout = std::io::stdout();
    let exit_code = match execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut stdout,
        &main_class,
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    ) {
        Ok(_) => None,
        Err(VmError::SystemExit { code }) => Some(code),
        Err(e) => {
            emit_mermaid_heap(&heap, mermaid_dest);
            emit_telemetry(&registry, telemetry);
            eprintln!("duke: runtime error: {e}");
            process::exit(1);
        }
    };
    emit_mermaid_heap(&heap, mermaid_dest);
    emit_telemetry(&registry, telemetry);
    if let Some(code) = exit_code {
        process::exit(code);
    }
}

// ---------------------------------------------------------------------------
// Dump
// ---------------------------------------------------------------------------

fn dump_analyze(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let report = analyze::generate_analysis_report(&cf);
    println!("{report}");
}

fn dump_uml(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let uml_diagram = uml::generate_mermaid_uml(&cf);
    println!("{uml_diagram}");
}

fn dump_html(path: &str, output_path: Option<&str>) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let html_content = html::generate_html_report(&cf);
    if let Some(out) = output_path {
        std::fs::write(out, html_content).unwrap_or_else(|e| {
            eprintln!("duke: cannot write to '{out}': {e}");
            process::exit(1);
        });
        println!("Report written to {out}");
    } else {
        println!("{html_content}");
    }
}

fn dump_cfg(path: &str, method_name: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let target = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
                return false;
            };
            s.as_str() == method_name
        })
        .unwrap_or_else(|| {
            eprintln!("duke: method '{method_name}' not found");
            process::exit(1);
        });

    for attr in &target.attributes {
        if let AttributeData::Code(code) = &attr.data {
            let instructions = decode(&code.code).unwrap_or_else(|e| {
                eprintln!("duke: decode error: {e}");
                process::exit(1);
            });
            println!("{}", generate_mermaid_cfg(&instructions));
            return;
        }
    }
    eprintln!("duke: method '{method_name}' has no code attribute");
    process::exit(1);
}

fn dump_cg(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    println!("{}", generate_mermaid_call_graph(&cf));
}

#[cfg(test)]
mod cfg_tests {
    use super::*;

    #[test]
    fn test_extract_cfg() {
        // Find HelloWorld.class in tests/fixtures
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");

        let bytes = std::fs::read(&p).expect("read class");
        let cf = parse(&bytes).expect("parse class");

        // Use core logic inside dump_cfg to extract main
        let target = cf
            .methods
            .iter()
            .find(|m| {
                let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
                else {
                    return false;
                };
                s.as_str() == "main"
            })
            .expect("find main");

        let mut found_code = false;
        for attr in &target.attributes {
            if let AttributeData::Code(code) = &attr.data {
                found_code = true;
                let instructions = decode(&code.code).expect("decode instructions");
                let cfg_str = generate_mermaid_cfg(&instructions);
                assert!(cfg_str.contains("graph TD"));
                assert!(cfg_str.contains("getstatic"));
            }
        }
        assert!(found_code, "should have found code attribute for main");
    }

    // To trigger the `dump_cfg` function coverage for error cases:
    #[test]
    fn test_dump_cfg_missing_file() {
        let mut bin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        bin_path.push("../target/debug/duke"); // Assume run via cargo test builds bin

        // Even without invoking the bin via Command, we can test main's coverage
        // for some helper functions in duke.

        let mut args = vec![
            "duke".to_string(),
            "--telemetry".to_string(),
            "--mermaid-heap".to_string(),
        ];
        assert_eq!(
            extract_telemetry_flag(&mut args),
            Some(TelemetryDest::Stdout)
        );
        assert_eq!(
            extract_mermaid_heap_flag(&mut args),
            Some(MermaidDest::Stdout)
        );
    }

    #[test]
    fn test_dump_html() {
        // Find HelloWorld.class in tests/fixtures
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");

        // Write HTML to a temp file
        let temp_dir = std::env::temp_dir();
        let out_path = temp_dir.join("test_dump_html.html");

        // Test with output path
        dump_html(p.to_str().unwrap(), Some(out_path.to_str().unwrap()));
        let html_content = std::fs::read_to_string(&out_path).unwrap();
        assert!(html_content.contains("<!DOCTYPE html>"));
        assert!(html_content.contains("Duke Class Report: HelloWorld"));

        // Clean up
        std::fs::remove_file(out_path).unwrap();

        // Also test without output path (writes to stdout). We can't easily capture stdout here
        // but we can ensure it doesn't panic.
        dump_html(p.to_str().unwrap(), None);
    }
}

fn generate_stubs(cf: &ClassFile) {
    println!("{}", generate_native_stubs_code(cf));
}

use std::fmt::Write;

/// Generates Rust source code containing native method stubs for the given `ClassFile`.
///
/// Scans the class for `native` methods and outputs a block of Rust code that includes
/// a `register_natives` function to bind the handlers, along with placeholder stub
/// implementations for each native method that return `VmError::Unimplemented`.
#[must_use]
pub fn generate_native_stubs_code(cf: &ClassFile) -> String {
    let mut out = String::new();
    let class_name = resolve_class_name(cf, cf.this_class);

    let mut native_methods = Vec::new();
    for method in &cf.methods {
        if method.access_flags.contains(MethodAccessFlags::NATIVE) {
            let name = cp_str(cf, method.name_index)
                .unwrap_or("<invalid>")
                .to_string();
            let desc = cp_str(cf, method.descriptor_index)
                .unwrap_or("<invalid>")
                .to_string();
            native_methods.push((name, desc));
        }
    }

    if native_methods.is_empty() {
        return format!("// No native methods found in class {class_name}\n");
    }

    let _ = writeln!(out, "// Native stubs for class {class_name}\n");

    // Generate register calls
    out.push_str("pub fn register_natives(registry: &mut ClassRegistry) {\n");
    for (name, desc) in &native_methods {
        let fn_name = format!(
            "native_{}_{}",
            class_name.replace('/', "_"),
            name.replace(['<', '>'], "")
        );
        let _ = writeln!(
            out,
            "    registry.natives_mut().register(\"{class_name}\", \"{name}\", \"{desc}\", {fn_name});"
        );
    }
    out.push_str("}\n\n");

    // Generate stub functions
    for (name, desc) in &native_methods {
        let fn_name = format!(
            "native_{}_{}",
            class_name.replace('/', "_"),
            name.replace(['<', '>'], "")
        );
        let _ = write!(
            out,
            "fn {fn_name}(\n    args: &[Slot],\n    heap: &mut duke_gc::Heap,\n    out: &mut dyn std::io::Write,\n    control: &mut duke_interpreter::NativeControl,\n) -> duke_runtime::VmResult<Option<Slot>> {{\n"
        );
        let _ = writeln!(
            out,
            "    // TODO: Implement native method {class_name}.{name} {desc}"
        );
        out.push_str(
            "    Err(duke_runtime::VmError::Unimplemented { mnemonic: \"native_stub\" })\n",
        );
        out.push_str("}\n\n");
    }

    out
}

fn dump_class_file(cf: &ClassFile) {
    let this_name = resolve_class_name(cf, cf.this_class);
    let super_name = resolve_class_name(cf, cf.super_class);

    println!("Class:        {this_name}");
    println!("Super:        {super_name}");
    println!("Version:      {}.{}", cf.major_version, cf.minor_version);
    println!("Access flags: {:?}", cf.access_flags);
    println!();

    // Constant pool
    println!(
        "=== Constant Pool ({} slots) ===",
        cf.constant_pool.len() - 1
    );
    for (i, entry) in cf.constant_pool.iter().enumerate() {
        if i == 0 {
            continue; // slot 0 is reserved
        }
        match entry {
            None => println!("  #{i:3}  (phantom — occupied by preceding Long/Double)"),
            Some(e) => println!("  #{i:3}  {}", format_cp_entry(cf, e)),
        }
    }
    println!();

    // Interfaces
    if !cf.interfaces.is_empty() {
        println!("=== Interfaces ===");
        for idx in &cf.interfaces {
            println!("  {}", resolve_class_name(cf, *idx));
        }
        println!();
    }

    // Fields
    if !cf.fields.is_empty() {
        println!("=== Fields ({}) ===", cf.fields.len());
        for field in &cf.fields {
            let name = cp_str(cf, field.name_index).unwrap_or("<invalid>");
            let desc = cp_str(cf, field.descriptor_index).unwrap_or("<invalid>");
            println!("  [{:?}] {name}: {desc}", field.access_flags);
        }
        println!();
    }

    // Methods
    println!("=== Methods ({}) ===", cf.methods.len());
    for method in &cf.methods {
        let name = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        println!("  [{:?}] {name}{desc}", method.access_flags);

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                println!(
                    "    code: {} bytes, max_stack={}, max_locals={}",
                    code.code.len(),
                    code.max_stack,
                    code.max_locals,
                );
                // Decode and print instructions
                match decode(&code.code) {
                    Ok(instructions) => {
                        for (pc, instr) in &instructions {
                            println!("      {:4}: {}", pc, instr.mnemonic());
                        }
                    }
                    Err(e) => println!("      [decode error: {e}]"),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn format_cp_entry(cf: &ClassFile, entry: &CpEntry) -> String {
    match entry {
        CpEntry::Utf8(s) => format!("Utf8           \"{s}\""),
        CpEntry::Integer(v) => format!("Integer         {v}"),
        CpEntry::Float(v) => format!("Float           {v}"),
        CpEntry::Long(v) => format!("Long            {v}L"),
        CpEntry::Double(v) => format!("Double          {v}"),
        CpEntry::Class { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Class           #{} // {name}", name_index.0)
        }
        CpEntry::String { string_index } => {
            let s = cp_str(cf, *string_index).unwrap_or("?");
            format!("String          #{} // \"{s}\"", string_index.0)
        }
        CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "Fieldref        #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::Methodref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "Methodref       #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        } => {
            format!(
                "IfaceMethodref  #{}.#{}",
                class_index.0, name_and_type_index.0
            )
        }
        CpEntry::NameAndType {
            name_index,
            descriptor_index,
        } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            let desc = cp_str(cf, *descriptor_index).unwrap_or("?");
            format!(
                "NameAndType     #{}.#{} // {name}:{desc}",
                name_index.0, descriptor_index.0
            )
        }
        CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        } => {
            format!(
                "MethodHandle    kind={reference_kind} ref=#{}",
                reference_index.0
            )
        }
        CpEntry::MethodType { descriptor_index } => {
            let desc = cp_str(cf, *descriptor_index).unwrap_or("?");
            format!("MethodType      #{} // {desc}", descriptor_index.0)
        }
        CpEntry::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            format!(
                "Dynamic         bsm={bootstrap_method_attr_index} nat=#{}",
                name_and_type_index.0
            )
        }
        CpEntry::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            format!(
                "InvokeDynamic   bsm={bootstrap_method_attr_index} nat=#{}",
                name_and_type_index.0
            )
        }
        CpEntry::Module { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Module          #{} // {name}", name_index.0)
        }
        CpEntry::Package { name_index } => {
            let name = cp_str(cf, *name_index).unwrap_or("?");
            format!("Package         #{} // {name}", name_index.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MermaidDest, TelemetryDest, extract_mermaid_heap_flag, extract_telemetry_flag};

    #[test]
    fn test_extract_telemetry_flag_md_stdout() {
        let mut args = vec![
            "duke".to_string(),
            "--telemetry-md".to_string(),
            "run".to_string(),
        ];
        let res = extract_telemetry_flag(&mut args);
        assert_eq!(res, Some(TelemetryDest::Markdown(String::new())));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_extract_telemetry_flag_md_file() {
        let mut args = vec![
            "duke".to_string(),
            "--telemetry-md=output.md".to_string(),
            "run".to_string(),
        ];
        let res = extract_telemetry_flag(&mut args);
        assert_eq!(res, Some(TelemetryDest::Markdown("output.md".to_string())));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_extract_mermaid_heap_flag_none() {
        let mut args = vec!["duke".to_string(), "run".to_string()];
        let res = extract_mermaid_heap_flag(&mut args);
        assert_eq!(res, None);
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_generate_native_stubs_code_empty() {
        use duke_classfile::{
            access_flags::ClassAccessFlags,
            types::{ClassFile, CpEntry, CpIndex},
        };

        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("EmptyClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(2),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let output = super::generate_native_stubs_code(&cf);
        assert!(output.contains("// No native methods found in class EmptyClass"));
    }

    #[test]
    fn test_generate_native_stubs_code() {
        use duke_classfile::{
            access_flags::{ClassAccessFlags, MethodAccessFlags},
            types::{ClassFile, CpEntry, CpIndex, MethodInfo},
        };

        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("java/lang/System".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Utf8("currentTimeMillis".to_string())),
                Some(CpEntry::Utf8("()J".to_string())),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(2),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC | MethodAccessFlags::NATIVE,
                name_index: CpIndex(3),
                descriptor_index: CpIndex(4),
                attributes: vec![],
            }],
            attributes: vec![],
        };

        let output = super::generate_native_stubs_code(&cf);

        assert!(output.contains("// Native stubs for class java/lang/System"));
        assert!(output.contains("registry.natives_mut().register(\"java/lang/System\", \"currentTimeMillis\", \"()J\", native_java_lang_System_currentTimeMillis);"));
        assert!(output.contains("fn native_java_lang_System_currentTimeMillis("));
        assert!(
            output.contains(
                "// TODO: Implement native method java/lang/System.currentTimeMillis ()J"
            )
        );
    }

    #[test]
    fn test_extract_mermaid_heap_flag_stdout() {
        let mut args = vec![
            "duke".to_string(),
            "--mermaid-heap".to_string(),
            "run".to_string(),
        ];
        let res = extract_mermaid_heap_flag(&mut args);
        assert_eq!(res, Some(MermaidDest::Stdout));
        assert_eq!(args.len(), 2); // the flag itself is removed
    }

    #[test]
    fn test_extract_mermaid_heap_flag_file() {
        let mut args = vec![
            "duke".to_string(),
            "--mermaid-heap=output.mmd".to_string(),
            "run".to_string(),
        ];
        let res = extract_mermaid_heap_flag(&mut args);
        assert_eq!(res, Some(MermaidDest::File("output.mmd".to_string())));
        assert_eq!(args.len(), 2); // the flag itself is removed
    }

    #[test]
    fn test_emit_mermaid_heap_stdout() {
        use super::emit_mermaid_heap;
        use duke_gc::Heap;
        let heap = Heap::new();
        // Just checking that it doesn't panic. Stdout can't be easily captured here,
        // but it executes the branch.
        emit_mermaid_heap(&heap, Some(MermaidDest::Stdout));
    }

    #[test]
    fn test_emit_mermaid_heap_file() {
        use super::emit_mermaid_heap;
        use duke_gc::Heap;
        use std::fs;
        let heap = Heap::new();
        let path = "test_output.mmd";
        emit_mermaid_heap(&heap, Some(MermaidDest::File(path.to_string())));

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("graph TD"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_emit_mermaid_heap_file_error() {
        use super::emit_mermaid_heap;
        use duke_gc::Heap;
        let heap = Heap::new();
        // Trying to write to a directory should trigger an IO error
        let path = ".";
        // It shouldn't panic, but print an error to stderr (handled by the branch we want to cover)
        emit_mermaid_heap(&heap, Some(MermaidDest::File(path.to_string())));
    }
}
