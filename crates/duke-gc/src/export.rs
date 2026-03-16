use crate::{Heap, OLD_BIT, Slot};
use std::fmt::Write;

/// Exports the heap to Graphviz DOT format for visualization.
pub struct HeapGraphExporter<'a> {
    heap: &'a Heap,
}

impl<'a> HeapGraphExporter<'a> {
    #[must_use]
    pub const fn new(heap: &'a Heap) -> Self {
        Self { heap }
    }

    /// Generate the DOT string.
    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "digraph Heap {{").unwrap();
        writeln!(&mut out, "  node [shape=box, fontname=\"Courier\"];").unwrap();
        writeln!(&mut out, "  rankdir=LR;").unwrap();

        writeln!(&mut out, "  subgraph cluster_young {{").unwrap();
        writeln!(&mut out, "    label = \"Young Generation\";").unwrap();
        writeln!(&mut out, "    style = dashed;").unwrap();
        for (i, slot) in self.heap.young.iter().enumerate() {
            if let Some(obj) = slot {
                Self::write_node(&mut out, i as u64, obj);
            }
        }
        writeln!(&mut out, "  }}").unwrap();

        writeln!(&mut out, "  subgraph cluster_old {{").unwrap();
        writeln!(&mut out, "    label = \"Old Generation\";").unwrap();
        writeln!(&mut out, "    style = dashed;").unwrap();
        for (i, slot) in self.heap.old.iter().enumerate() {
            if let Some(obj) = slot {
                Self::write_node(&mut out, (i as u64) | OLD_BIT, obj);
            }
        }
        writeln!(&mut out, "  }}").unwrap();

        for (i, slot) in self.heap.young.iter().enumerate() {
            if let Some(obj) = slot {
                Self::write_edges(&mut out, i as u64, obj);
            }
        }
        for (i, slot) in self.heap.old.iter().enumerate() {
            if let Some(obj) = slot {
                Self::write_edges(&mut out, (i as u64) | OLD_BIT, obj);
            }
        }

        writeln!(&mut out, "}}").unwrap();
        out
    }

    fn write_node(out: &mut String, ref_id: u64, obj: &crate::HeapObject) {
        let age_str = if obj.age > 0 {
            format!(" (age: {})", obj.age)
        } else {
            String::new()
        };
        let mut display = format!("{c}{age_str}", c = obj.class_name);
        if let Some(s) = &obj.string_value {
            let escaped = s.replace('"', "\\\"");
            let _ = write!(display, "\\n\\\"{escaped}\\\"");
        }
        writeln!(out, "    obj_{ref_id} [label=\"{display}\"];").unwrap();
    }

    fn write_edges(out: &mut String, ref_id: u64, obj: &crate::HeapObject) {
        for (idx, field) in obj.fields.iter().enumerate() {
            if let Slot::Reference(Some(target_r)) = field {
                writeln!(out, "  obj_{ref_id} -> obj_{target_r} [label=\"f{idx}\"];").unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_dot_empty_heap() {
        let heap = Heap::new();
        let exporter = HeapGraphExporter::new(&heap);
        let dot = exporter.to_dot();
        assert!(dot.contains("digraph Heap"));
        assert!(dot.contains("cluster_young"));
        assert!(dot.contains("cluster_old"));
    }

    #[test]
    fn test_to_dot_with_objects() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("java/lang/Object".to_string(), 1);
        let r1 = heap.allocate_string("hello".to_string());
        heap.get_mut(r0).unwrap().fields[0] = Slot::Reference(Some(r1));

        let exporter = HeapGraphExporter::new(&heap);
        let dot = exporter.to_dot();

        assert!(dot.contains("java/lang/Object"));
        assert!(dot.contains("\\\"hello\\\""));
        assert!(dot.contains("obj_0 -> obj_1 [label=\"f0\"];"));
    }
}
