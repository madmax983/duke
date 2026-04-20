with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

# We want to replace the current doc string of `execute_class`
# Currently it looks like this:
target = '''/// let res = execute_class(&mut registry, &loader, &mut heap, &mut out, "java/lang/Object", "hashCode", "()I", &[]);
/// assert!(res.is_err());
/// ```
/// Bootstraps and executes a specific method on a Java class.
///
/// This handles the setup of initial JVM state, resolution of the target
/// class and method, and initiates the interpretation loop.
///
/// # Arguments
///
/// * `registry` - The loaded class registry.
/// * `loader` - The class loader instance.
/// * `heap` - The garbage-collected heap.
/// * `stdout` - The standard output writer.
/// * `class_name` - The fully qualified name of the target class.
/// * `method_name` - The name of the target method.
/// * `descriptor` - The type descriptor of the target method.
/// * `args` - The arguments to pass to the method.
///
/// # Errors
///
/// Returns `VmError` if execution fails or the class/method cannot be resolved.
#[allow('''

new_doc = '''/// Bootstraps and executes a specific method on a Java class.
///
/// This handles the setup of initial JVM state, resolution of the target
/// class and method, and initiates the interpretation loop.
///
/// # Arguments
///
/// * `registry` - The loaded class registry.
/// * `loader` - The class loader instance.
/// * `heap` - The garbage-collected heap.
/// * `stdout` - The standard output writer.
/// * `class_name` - The fully qualified name of the target class.
/// * `method_name` - The name of the target method.
/// * `descriptor` - The type descriptor of the target method.
/// * `args` - The arguments to pass to the method.
///
/// # Errors
///
/// Returns `VmError` if execution fails or the class/method cannot be resolved.
///
/// # Examples
///
/// ```
/// use duke_interpreter::{ClassRegistry, execute_class};
/// use duke_gc::Heap;
/// use duke_loader::BootstrapLoader;
/// let mut registry = ClassRegistry::new();
/// let loader = BootstrapLoader::new(vec![]);
/// let mut heap = Heap::new();
/// let mut out = std::io::stdout();
/// // Try to execute java/lang/Object.hashCode() without allocating an object first
/// let res = execute_class(&mut registry, &loader, &mut heap, &mut out, "java/lang/Object", "hashCode", "()I", &[]);
/// assert!(res.is_err());
/// ```
#[allow('''

# Note: The original doc without my append had the example.
# Let's read the file again to find what it exactly looks like
