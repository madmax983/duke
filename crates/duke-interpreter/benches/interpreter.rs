#![allow(missing_docs)]

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use duke_interpreter::{ClassRegistry, bootstrap_stdlib, build_class_context, execute_class};
use duke_loader::DirectoryLoader;
use std::path::PathBuf;

/// Path to the `benchmarks/` directory at the workspace root.
fn bench_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("benchmarks")
}

/// Set up a fresh registry + heap + loader for one benchmark run.
/// Includes: parse .class, register class context, bootstrap stdlib.
fn make_env() -> (ClassRegistry, duke_gc::Heap, DirectoryLoader) {
    let dir = bench_dir();
    let bytes = std::fs::read(dir.join("BenchmarkSuite.class"))
        .expect("benchmarks/BenchmarkSuite.class not found — run Task 1 first");
    let cf = duke_classfile::parse(&bytes).unwrap();
    let ctx = build_class_context(&cf);
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = DirectoryLoader::new(&dir);
    (registry, heap, loader)
}

fn bench_sum(c: &mut Criterion) {
    c.bench_function("benchSum (500k int adds)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchSum",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_fib(c: &mut Criterion) {
    c.bench_function("benchFib (fib(25), ~500k calls)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchFib",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_arraylist(c: &mut Criterion) {
    c.bench_function("benchArrayList (5k ArrayList.add)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchArrayList",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_hashmap(c: &mut Criterion) {
    c.bench_function("benchHashMap (200 put + 200 get)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchHashMap",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_bootstrap_only(c: &mut Criterion) {
    c.bench_function("bootstrap_stdlib only", |b| {
        b.iter(|| {
            let mut registry = ClassRegistry::new();
            let mut heap = duke_gc::Heap::new();
            bootstrap_stdlib(&mut registry, &mut heap);
        });
    });
}

criterion_group!(
    benches,
    bench_sum,
    bench_fib,
    bench_arraylist,
    bench_hashmap,
    bench_bootstrap_only
);
criterion_main!(benches);
