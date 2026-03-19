use crate::{Heap, OLD_BIT};
use duke_runtime::Slot;
use std::fmt::Write;

impl Heap {
    /// Generate a Mermaid class diagram representing the JVM heap.
    ///
    /// The diagram will include subgraphs for the Young and Old generations,
    /// showing individual objects and their references to other objects.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        let mut mermaid = String::new();
        writeln!(&mut mermaid, "classDiagram").unwrap();
        writeln!(&mut mermaid, "    direction LR").unwrap();

        let mut young_nodes = Vec::new();
        let mut old_nodes = Vec::new();
        let mut relationships = Vec::new();

        // Process Young Generation
        for (i, slot) in self.young.iter().enumerate() {
            if let Some(obj) = slot {
                let node_id = format!("Y{i}");
                let label = obj.string_value.as_ref().map_or_else(|| format!("{} (Age: {})", obj.class_name, obj.age), |s| format!(
                        "{} (Age: {}) \\\"{}\"",
                        obj.class_name,
                        obj.age,
                        s.replace('"', "\\\"")
                    ));
                young_nodes.push(format!("        class {node_id}[\"{label}\"]"));

                for (field_idx, field_slot) in obj.fields.iter().enumerate() {
                    if let Slot::Reference(Some(r)) = field_slot {
                        let target_id = if r & OLD_BIT != 0 {
                            format!("O{}", r & !OLD_BIT)
                        } else {
                            format!("Y{r}")
                        };
                        relationships.push(format!(
                            "    {node_id} --> {target_id} : field\\[{field_idx}\\]",
                        ));
                    }
                }
            }
        }

        // Process Old Generation
        for (i, slot) in self.old.iter().enumerate() {
            if let Some(obj) = slot {
                let node_id = format!("O{i}");
                let label = obj.string_value.as_ref().map_or_else(|| obj.class_name.clone(), |s| format!(
                    "{} \\\"{}\"",
                    obj.class_name,
                    s.replace('"', "\\\"")
                ));
                old_nodes.push(format!("        class {node_id}[\"{label}\"]"));

                for (field_idx, field_slot) in obj.fields.iter().enumerate() {
                    if let Slot::Reference(Some(r)) = field_slot {
                        let target_id = if r & OLD_BIT != 0 {
                            format!("O{}", r & !OLD_BIT)
                        } else {
                            format!("Y{r}")
                        };
                        relationships.push(format!(
                            "    {node_id} --> {target_id} : field\\[{field_idx}\\]",
                        ));
                    }
                }
            }
        }

        if !young_nodes.is_empty() {
            writeln!(&mut mermaid, "    namespace YoungGen {{").unwrap();
            for node in young_nodes {
                writeln!(&mut mermaid, "{node}").unwrap();
            }
            writeln!(&mut mermaid, "    }}").unwrap();
        }

        if !old_nodes.is_empty() {
            writeln!(&mut mermaid, "    namespace OldGen {{").unwrap();
            for node in old_nodes {
                writeln!(&mut mermaid, "{node}").unwrap();
            }
            writeln!(&mut mermaid, "    }}").unwrap();
        }

        for rel in relationships {
            writeln!(&mut mermaid, "{rel}").unwrap();
        }

        mermaid
    }
}

#[cfg(test)]
mod tests {
    use crate::{Heap, OLD_BIT};
    use duke_runtime::Slot;

    #[test]
    fn test_to_mermaid_empty() {
        let heap = Heap::new();
        let expected = "classDiagram\n    direction LR\n";
        assert_eq!(heap.to_mermaid(), expected);
    }

    #[test]
    fn test_to_mermaid_with_objects() {
        let mut heap = Heap::new();

        let root = heap.allocate("RootNode".to_string(), 2);
        let child1 = heap.allocate("ChildNode".to_string(), 0);
        let child2 = heap.allocate("ChildNode".to_string(), 0);

        heap.write_field(root, 0, Slot::Reference(Some(child1)))
            .unwrap();
        heap.write_field(root, 1, Slot::Reference(Some(child2)))
            .unwrap();

        let mermaid = heap.to_mermaid();

        assert!(mermaid.contains("namespace YoungGen"));
        assert!(mermaid.contains("class Y0[\"RootNode (Age: 0)\"]"));
        assert!(mermaid.contains("class Y1[\"ChildNode (Age: 0)\"]"));
        assert!(mermaid.contains("class Y2[\"ChildNode (Age: 0)\"]"));

        assert!(!mermaid.contains("namespace OldGen"));

        assert!(mermaid.contains("Y0 --> Y1 : field\\[0\\]"));
        assert!(mermaid.contains("Y0 --> Y2 : field\\[1\\]"));
    }

    #[test]
    fn test_to_mermaid_with_old_objects() {
        let mut heap = Heap::new();

        let root = heap.allocate("RootNode".to_string(), 1);
        let child = heap.allocate("ChildNode".to_string(), 0);
        heap.write_field(root, 0, Slot::Reference(Some(child)))
            .unwrap();

        // Simulate promotion by directly moving to old gen and updating reference
        let child_obj = heap.young[usize::try_from(child).unwrap()].take().unwrap();
        heap.old.push(Some(child_obj));
        let new_child_ref = (heap.old.len() as u64 - 1) | OLD_BIT;
        heap.write_field(root, 0, Slot::Reference(Some(new_child_ref)))
            .unwrap();

        let mermaid = heap.to_mermaid();

        assert!(mermaid.contains("namespace YoungGen"));
        assert!(mermaid.contains("class Y0[\"RootNode (Age: 0)\"]"));

        assert!(mermaid.contains("namespace OldGen"));
        assert!(mermaid.contains("class O0[\"ChildNode\"]"));

        assert!(mermaid.contains("Y0 --> O0 : field\\[0\\]"));
    }
}
