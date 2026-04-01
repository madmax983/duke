#[cfg(test)]
mod tests {
    use duke_interpreter::{ClassRegistry, bootstrap_stdlib, execute_class_to_completion};
    use duke_loader::{ClassLoader, DirectoryLoader};

    struct FixtureLoader(DirectoryLoader);
    impl ClassLoader for FixtureLoader {
        fn find_class(&self, name: &str) -> duke_loader::LoadResult<Vec<u8>> {
            self.0.find_class(name)
        }
    }

    #[test]
    fn havoc_self_join_panics() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixtures_dir = std::path::Path::new(manifest_dir).join("../../tests/fixtures");

        let loader = FixtureLoader(DirectoryLoader::new(&fixtures_dir));

        let mut output = Vec::new();

        let err = execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut output,
            "ThreadingTest",
            "selfJoinPanic",
            "()I",
            &[],
        );
        println!("{err:?}");
    }
}
