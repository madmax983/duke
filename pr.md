⚒️ Forge: refactor let-some bindings in call_graph and fix jar_diff imports

🚮 Smell: `generate_mermaid_call_graph` had a complex `if let ... && let ...` nesting (Let Chains) leading to a "Pyramid of Doom". Additionally, `jar_diff.rs` had unresolved imports and missing type annotations on closures.
✨ Solution: Used early returns `let Some(x) = y else { continue; }` to flatten the deep nesting in `generate_mermaid_call_graph`. Fixed the unresolved `types` import in `jar_diff.rs` and added explicit closure type annotations to satisfy the compiler.
🧼 Benefit: Significantly flattens code by replacing pyramids of doom with guard clauses, improving readability and cognitive load. Fixes compiler errors.
🛡️ Verification: Tests passed. No logic changed.
