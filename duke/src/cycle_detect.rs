use duke_classfile::{
    parse,
    types::{CpEntry, CpIndex},
};
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process;

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
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

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

/// Analyzes a JAR file to detect circular class dependencies.
///
/// **Why it exists:** Architectural decay often manifests as circular dependencies
/// between classes. This utility scans the entire JAR, builds a global dependency
/// graph, and uses Tarjan's strongly connected components algorithm to find cycles.
#[cfg(feature = "nova")]
#[allow(clippy::items_after_statements)]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs, dead_code)]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::collapsible_if,
    clippy::too_many_arguments
)]
pub fn dump_cycle_detect(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();

    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();

    for entry_name in reader.entry_names() {
        if !std::path::Path::new(&entry_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("class"))
        {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_name = resolve_class_name(&cf, cf.this_class);
                let deps = graph.entry(this_name.clone()).or_default();

                for (i, entry) in cf.constant_pool.iter().enumerate() {
                    if let Some(CpEntry::Class { .. }) = entry {
                        let idx = CpIndex(u16::try_from(i).unwrap_or(u16::MAX));
                        if idx != cf.this_class {
                            let ref_name = resolve_class_name(&cf, idx);
                            if ref_name != "<invalid utf8>"
                                && ref_name != "<not a class ref>"
                                && ref_name != "<none>"
                            {
                                deps.insert(ref_name);
                            }
                        }
                    }
                }
            }
        }
    }

    // Tarjan's SCC
    let mut index = 0;
    let mut stack: Vec<String> = Vec::new();
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlinks: HashMap<String, usize> = HashMap::new();
    let mut on_stack: HashSet<String> = HashSet::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        v: &str,
        graph: &HashMap<String, HashSet<String>>,
        index: &mut usize,
        stack: &mut Vec<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        on_stack: &mut HashSet<String>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(v.to_string(), *index);
        lowlinks.insert(v.to_string(), *index);
        *index += 1;
        stack.push(v.to_string());
        on_stack.insert(v.to_string());

        if let Some(edges) = graph.get(v) {
            for w in edges {
                // We only care about cycles within the JAR classes, so if w isn't in graph keys, ignore.
                if !graph.contains_key(w) {
                    continue;
                }
                if !indices.contains_key(w) {
                    strongconnect(w, graph, index, stack, indices, lowlinks, on_stack, sccs);
                    let min_low =
                        std::cmp::min(*lowlinks.get(v).unwrap(), *lowlinks.get(w).unwrap());
                    lowlinks.insert(v.to_string(), min_low);
                } else if on_stack.contains(w) {
                    let min_low =
                        std::cmp::min(*lowlinks.get(v).unwrap(), *indices.get(w).unwrap());
                    lowlinks.insert(v.to_string(), min_low);
                }
            }
        }

        if lowlinks.get(v) == indices.get(v) {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                scc.push(w.clone());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                sccs.push(scc);
            }
        }
    }

    for v in graph.keys() {
        if !indices.contains_key(v) {
            strongconnect(
                v,
                &graph,
                &mut index,
                &mut stack,
                &mut indices,
                &mut lowlinks,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    println!("======================================");
    println!(" JAR Dependency Cycle Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Total Classes:        {}", graph.len());
    println!("Total Cycles Found:   {}", sccs.len());
    println!();

    if sccs.is_empty() {
        println!("✅ No dependency cycles detected. Great architecture!");
    } else {
        println!("🚨 Cyclical dependencies detected:");
        for (i, scc) in sccs.iter().enumerate() {
            println!("  Cycle {}:", i + 1);
            for node in scc {
                println!("    - {node}");
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "nova")]
    fn test_dump_cycle_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Just ensure it runs without panicking on a valid JAR.
        super::dump_cycle_detect(path_str);
    }
}
