patch -p1 << 'PATCH_EOF'
--- a/crates/duke-interpreter/src/native/java_util_regex.rs
+++ b/crates/duke-interpreter/src/native/java_util_regex.rs
@@ -85,15 +85,16 @@
     _control: &mut NativeControl,
 ) -> Result<Option<Slot>> {
     let m_ref = extract_ref_arg(args, 0)?;
-    let fields = heap.get(m_ref)?.fields.clone();
-    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
-        return Ok(Some(Slot::Int(0)));
-    };
-    let input_slot = fields
-        .get(MATCHER_INPUT_FIELD)
-        .copied()
-        .unwrap_or(Slot::Reference(None));
-    let Slot::Reference(Some(input_ref)) = input_slot else {
-        return Ok(Some(Slot::Int(0)));
-    };
-    let pos = match fields.get(MATCHER_POS_FIELD).copied() {
-        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
-        _ => 0,
-    };
+    let (pat_ref, input_ref, pos) = {
+        let fields = &heap.get(m_ref)?.fields;
+        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
+            return Ok(Some(Slot::Int(0)));
+        };
+        let input_slot = fields
+            .get(MATCHER_INPUT_FIELD)
+            .copied()
+            .unwrap_or(Slot::Reference(None));
+        let Slot::Reference(Some(input_ref)) = input_slot else {
+            return Ok(Some(Slot::Int(0)));
+        };
+        let pos = match fields.get(MATCHER_POS_FIELD).copied() {
+            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
+            _ => 0,
+        };
+        (pat_ref, input_ref, pos)
+    };
     let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
@@ -119,8 +120,9 @@
     _control: &mut NativeControl,
 ) -> Result<Option<Slot>> {
     let m_ref = extract_ref_arg(args, 0)?;
-    let fields = heap.get(m_ref)?.fields.clone();
-    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
-        return Ok(Some(Slot::Int(0)));
-    };
-    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
-        return Ok(Some(Slot::Int(0)));
-    };
+    let (pat_ref, input_ref) = {
+        let fields = &heap.get(m_ref)?.fields;
+        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
+            return Ok(Some(Slot::Int(0)));
+        };
+        let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
+            return Ok(Some(Slot::Int(0)));
+        };
+        (pat_ref, input_ref)
+    };
     let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
@@ -358,8 +360,9 @@
 ) -> Result<Option<Slot>> {
     let m_ref = extract_ref_arg(args, 0)?;
     let repl_ref = extract_ref_arg(args, 1)?;
-    let fields = heap.get(m_ref)?.fields.clone();
-    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
-        return Ok(Some(Slot::Reference(None)));
-    };
-    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
-        return Ok(Some(Slot::Reference(None)));
-    };
+    let (pat_ref, input_ref) = {
+        let fields = &heap.get(m_ref)?.fields;
+        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
+            return Ok(Some(Slot::Reference(None)));
+        };
+        let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
+            return Ok(Some(Slot::Reference(None)));
+        };
+        (pat_ref, input_ref)
+    };
     let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
@@ -382,8 +385,9 @@
 ) -> Result<Option<Slot>> {
     let m_ref = extract_ref_arg(args, 0)?;
     let repl_ref = extract_ref_arg(args, 1)?;
-    let fields = heap.get(m_ref)?.fields.clone();
-    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
-        return Ok(Some(Slot::Reference(None)));
-    };
-    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
-        return Ok(Some(Slot::Reference(None)));
-    };
+    let (pat_ref, input_ref) = {
+        let fields = &heap.get(m_ref)?.fields;
+        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
+            return Ok(Some(Slot::Reference(None)));
+        };
+        let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
+            return Ok(Some(Slot::Reference(None)));
+        };
+        (pat_ref, input_ref)
+    };
     let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
PATCH_EOF
