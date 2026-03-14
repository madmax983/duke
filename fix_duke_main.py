with open('duke/src/main.rs', 'r') as f:
    data = f.read()

data = data.replace('BootstrapLoader (JDK', '`BootstrapLoader` (JDK')
data = data.replace('plain DirectoryLoader otherwise.', 'plain `DirectoryLoader` otherwise.')

data = data.replace('eprintln!("duke: warning: {modules:?} not found, falling back to directory loader");', 'eprintln!("duke: warning: {} not found, falling back to directory loader", modules.display());')

data = data.replace('eprintln!("duke: cannot read \'{}\': {e}", path);', 'eprintln!("duke: cannot read \'{path}\': {e}");')
data = data.replace('eprintln!("duke: parse error in \'{}\': {e}", path);', 'eprintln!("duke: parse error in \'{path}\': {e}");')

data = data.replace('fn emit_telemetry(_registry: &ClassRegistry, dest: Option<TelemetryDest>) {', 'fn emit_telemetry(_registry: &ClassRegistry, dest: Option<&TelemetryDest>) {')
data = data.replace('fn emit_telemetry(registry: &ClassRegistry, dest: Option<TelemetryDest>) {', 'fn emit_telemetry(registry: &ClassRegistry, dest: Option<&TelemetryDest>) {')
data = data.replace('emit_telemetry(&registry, telemetry);', 'emit_telemetry(&registry, telemetry.as_ref());')

data = data.replace('.unwrap_or(std::path::Path::new("."));', '.unwrap_or_else(|| std::path::Path::new("."));')

with open('duke/src/main.rs', 'w') as f:
    f.write(data)
with open('duke/src/main.rs', 'r') as f:
    data = f.read()

data = data.replace('fn exec_method(args: &[String], telemetry: Option<TelemetryDest>, jdk_home: Option<&str>) {', 'fn exec_method(args: &[String], telemetry: Option<&TelemetryDest>, jdk_home: Option<&str>) {')
data = data.replace('fn run_main(args: &[String], telemetry: Option<TelemetryDest>, jdk_home: Option<&str>) {', 'fn run_main(args: &[String], telemetry: Option<&TelemetryDest>, jdk_home: Option<&str>) {')
data = data.replace('exec_method(&args[2..], telemetry, jdk_home.as_deref());', 'exec_method(&args[2..], telemetry.as_ref(), jdk_home.as_deref());')
data = data.replace('run_main(&args[2..], telemetry, jdk_home.as_deref());', 'run_main(&args[2..], telemetry.as_ref(), jdk_home.as_deref());')


with open('duke/src/main.rs', 'w') as f:
    f.write(data)
with open('duke/src/main.rs', 'r') as f:
    data = f.read()

data = data.replace('emit_telemetry(&registry, telemetry.as_ref());', 'emit_telemetry(&registry, telemetry);')

with open('duke/src/main.rs', 'w') as f:
    f.write(data)
