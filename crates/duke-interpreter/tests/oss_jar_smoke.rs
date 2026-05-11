//! Integration tests for running real-world OSS JARs.
//! These tests verify that Duke can successfully load, initialize, and execute
//! unmodified bytecode from third-party libraries like slf4j.

use std::path::{Path, PathBuf};

use duke_gc::Heap;
use duke_interpreter::{
    ClassLoadSource, ClassRegistry, bootstrap_stdlib, execute_class_to_completion,
};
use duke_loader::{BootstrapLoader, ClassLoader, ClasspathEntry, DirectoryLoader, ZipLoader};
use duke_runtime::{Error, Slot};

struct ChainLoader(Vec<ClasspathEntry>);

impl ClassLoader for ChainLoader {
    fn find_class(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        for entry in &self.0 {
            if let Ok(bytes) = entry.find_class(name) {
                return Ok(bytes);
            }
        }
        Err(duke_loader::Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        for entry in &self.0 {
            match entry.find_resource(name) {
                Err(duke_loader::Error::NotFound { .. }) => {}
                result => return result,
            }
        }
        Err(duke_loader::Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resources(&self, name: &str) -> duke_loader::Result<Vec<Vec<u8>>> {
        let mut resources = Vec::new();
        for entry in &self.0 {
            resources.extend(entry.find_resources(name)?);
        }
        Ok(resources)
    }

    fn find_resource_entry(&self, name: &str) -> duke_loader::Result<duke_loader::LocatedResource> {
        for entry in &self.0 {
            match entry.find_resource_entry(name) {
                Err(duke_loader::Error::NotFound { .. }) => {}
                result => return result,
            }
        }
        Err(duke_loader::Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource_entries(
        &self,
        name: &str,
    ) -> duke_loader::Result<Vec<duke_loader::LocatedResource>> {
        let mut resources = Vec::new();
        for entry in &self.0 {
            resources.extend(entry.find_resource_entries(name)?);
        }
        Ok(resources)
    }
}

enum OssSmokeLoader {
    Bootstrap(BootstrapLoader),
    Chain(ChainLoader),
}

impl ClassLoader for OssSmokeLoader {
    fn find_class(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        match self {
            Self::Bootstrap(loader) => loader.find_class(name),
            Self::Chain(loader) => loader.find_class(name),
        }
    }

    fn find_resource(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        match self {
            Self::Bootstrap(loader) => loader.find_resource(name),
            Self::Chain(loader) => loader.find_resource(name),
        }
    }

    fn find_resources(&self, name: &str) -> duke_loader::Result<Vec<Vec<u8>>> {
        match self {
            Self::Bootstrap(loader) => loader.find_resources(name),
            Self::Chain(loader) => loader.find_resources(name),
        }
    }

    fn find_resource_entry(&self, name: &str) -> duke_loader::Result<duke_loader::LocatedResource> {
        match self {
            Self::Bootstrap(loader) => loader.find_resource_entry(name),
            Self::Chain(loader) => loader.find_resource_entry(name),
        }
    }

    fn find_resource_entries(
        &self,
        name: &str,
    ) -> duke_loader::Result<Vec<duke_loader::LocatedResource>> {
        match self {
            Self::Bootstrap(loader) => loader.find_resource_entries(name),
            Self::Chain(loader) => loader.find_resource_entries(name),
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical workspace root")
}

fn fixtures_dir() -> PathBuf {
    repo_root().join("tests/fixtures")
}

fn oss_jars_dir() -> PathBuf {
    fixtures_dir().join("oss-jars")
}

fn oss_classpath() -> Vec<PathBuf> {
    vec![
        fixtures_dir(),
        oss_jars_dir().join("slf4j-api-2.0.13.jar"),
        oss_jars_dir().join("slf4j-simple-2.0.13.jar"),
    ]
}

fn jdk_modules_path() -> Option<PathBuf> {
    let from_java_home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("lib/modules"));
    let fallbacks = [
        PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64/lib/modules"),
        PathBuf::from("/usr/lib/jvm/default-java/lib/modules"),
        PathBuf::from("C:/Program Files/Java/jdk-21/lib/modules"),
    ];
    from_java_home
        .into_iter()
        .chain(fallbacks)
        .find(|path| path.exists())
}

fn classpath_entry(path: &Path) -> ClasspathEntry {
    let is_archive = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"));
    if is_archive {
        ClasspathEntry::Zip(ZipLoader::open(path).expect("open vendored jar"))
    } else {
        ClasspathEntry::Directory(DirectoryLoader::new(path))
    }
}

fn oss_smoke_loader() -> OssSmokeLoader {
    let classpath = oss_classpath();
    if let Some(modules_path) = jdk_modules_path()
        && let Ok(loader) = BootstrapLoader::new(&modules_path, classpath.clone())
    {
        return OssSmokeLoader::Bootstrap(loader);
    }

    let entries = classpath
        .iter()
        .map(|path| classpath_entry(path))
        .collect::<Vec<_>>();
    OssSmokeLoader::Chain(ChainLoader(entries))
}

struct SmokeRun {
    result: Result<Option<Slot>, Error>,
    output: String,
    registry: ClassRegistry,
}

fn render_smoke_error(err: &Error) -> String {
    match err {
        Error::MethodNotFound { name, descriptor } => {
            format!("Unsupported native: {name}{descriptor}")
        }
        Error::ClassNotFound { name } => format!("Missing class: {name}"),
        Error::Unimplemented { mnemonic } => format!("Unimplemented opcode: {mnemonic}"),
        other => format!("{other:?}"),
    }
}

fn run_slf4j_simple_method(method_name: &str, descriptor: &str) -> SmokeRun {
    let loader = oss_smoke_loader();
    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    assert!(
        registry
            .ensure_loaded("Slf4jSimpleSmoke", &loader)
            .expect("load Slf4jSimpleSmoke"),
        "Slf4jSimpleSmoke should load from the fixture classpath"
    );

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = [Slot::Reference(Some(args_ref))];
    let mut output = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut output,
        "Slf4jSimpleSmoke",
        method_name,
        descriptor,
        if descriptor == "([Ljava/lang/String;)V" {
            &main_args
        } else {
            &[]
        },
    );

    SmokeRun {
        result,
        output: String::from_utf8(output).expect("captured output is utf8"),
        registry,
    }
}

fn run_slf4j_simple_smoke() -> SmokeRun {
    run_slf4j_simple_method("main", "([Ljava/lang/String;)V")
}

#[test]
fn slf4j_simple_smoke_surfaces_next_missing_capability_explicitly() {
    let smoke = run_slf4j_simple_smoke();
    let err = smoke
        .result
        .expect_err("slf4j smoke should still hit the next unsupported capability");
    let rendered = render_smoke_error(&err);

    assert!(
        !rendered.contains("java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;"),
        "smoke should progress past getSecurityManager, got: {rendered}"
    );
    assert!(
        rendered.contains("Missing class: java/security/AccessController"),
        "expected explicit next missing capability, got: {rendered}"
    );
}

#[test]
#[ignore = "Blocked on java/security/AccessController after issue #687."]
fn slf4j_simple_smoke_runs_real_jar_bytecode() {
    let smoke = run_slf4j_simple_smoke();

    assert!(
        smoke.result.is_ok(),
        "Slf4jSimpleSmoke.main should execute end-to-end, got {}\nCaptured output:\n{}",
        smoke
            .result
            .as_ref()
            .err()
            .map_or_else(|| "no error".to_string(), render_smoke_error),
        smoke.output
    );

    assert!(
        smoke.output.contains("INFO duke-smoke - hello world"),
        "expected slf4j-simple log line in captured output, got: {}",
        smoke.output
    );

    let simple_logger = smoke
        .registry
        .get("org/slf4j/simple/SimpleLogger")
        .expect("SimpleLogger should be loaded");
    assert_eq!(simple_logger.load_source, ClassLoadSource::Classfile);

    let provider = smoke
        .registry
        .get("org/slf4j/simple/SimpleServiceProvider")
        .expect("SimpleServiceProvider should be loaded");
    assert_eq!(provider.load_source, ClassLoadSource::Classfile);
}

#[test]
#[ignore = "Probe for issue #663; keep ignored until the broader slf4j smoke moves forward."]
fn slf4j_simple_properties_lookup_returns_null_cleanly() {
    let probe = run_slf4j_simple_method("simpleLoggerPropertiesStreamIsNull", "()I");

    assert!(
        probe.result.is_ok(),
        "simplelogger.properties probe should execute without unsupported natives, got {}\nCaptured output:\n{}",
        probe
            .result
            .as_ref()
            .err()
            .map_or_else(|| "no error".to_string(), render_smoke_error),
        probe.output
    );

    assert_eq!(probe.result.expect("probe result"), Some(Slot::Int(1)));
}
