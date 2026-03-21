use crate::{Heap, OLD_BIT};
use duke_runtime::Slot;
use std::fmt::Write;

/// Generates a Mermaid JS graph of the heap.
pub fn dump_mermaid(heap: &Heap) -> String {
    let mut out = String::new();
    out.push_str("graph TD\n");

    // Helper to format a node ID based on its reference
    let node_id = |r: u64| -> String { format!("N{r}") };

    // Iterate over young gen
    for (i, opt_obj) in heap.young.iter().enumerate() {
        if let Some(obj) = opt_obj {
            let r = i as u64;
            let class_name = &obj.class_name;
            if let Some(s) = &obj.string_value {
                let escaped = s.replace('"', "\\\"");
                let _ = writeln!(
                    out,
                    "    {}[{} \\\"{}\\\"]",
                    node_id(r),
                    class_name,
                    escaped
                );
            } else {
                let _ = writeln!(out, "    {}[{}]", node_id(r), class_name);
            }

            for field in &obj.fields {
                if let Slot::Reference(Some(target_r)) = field {
                    let _ = writeln!(out, "    {} --> {}", node_id(r), node_id(*target_r));
                }
            }
        }
    }

    // Iterate over old gen
    for (i, opt_obj) in heap.old.iter().enumerate() {
        if let Some(obj) = opt_obj {
            let r = (i as u64) | OLD_BIT;
            let class_name = &obj.class_name;
            if let Some(s) = &obj.string_value {
                let escaped = s.replace('"', "\\\"");
                let _ = writeln!(
                    out,
                    "    {}[{} \\\"{}\\\"]",
                    node_id(r),
                    class_name,
                    escaped
                );
            } else {
                let _ = writeln!(out, "    {}[{}]", node_id(r), class_name);
            }

            for field in &obj.fields {
                if let Slot::Reference(Some(target_r)) = field {
                    let _ = writeln!(out, "    {} --> {}", node_id(r), node_id(*target_r));
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dump_mermaid() {
        let mut heap = Heap::new();
        let r1 = heap.allocate("java/lang/Object".to_string(), 1);
        let r2 = heap.allocate_string("hello mermaid".to_string());
        heap.write_field(r1, 0, Slot::Reference(Some(r2))).unwrap();

        let mermaid = dump_mermaid(&heap);

        assert!(mermaid.starts_with("graph TD\n"));
        // Check node definitions
        assert!(mermaid.contains("N0[java/lang/Object]"));
        assert!(mermaid.contains("N1[java/lang/String \\\"hello mermaid\\\"]"));
        // Check edge
        assert!(mermaid.contains("N0 --> N1"));
    }
}
