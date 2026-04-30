# java.util.Properties Implementation Note

## Scope

Issue #649 adds a synthetic `java/util/Properties` surface in `duke-interpreter` for the common `InputStream`-based configuration path:

- constructors with and without a defaults chain
- `setProperty` / `getProperty` overloads
- ISO-8859-1 `load(InputStream)` parsing with Java properties escapes
- deterministic `store(OutputStream, String)` output
- local map views plus merged `propertyNames` / `stringPropertyNames`

## Class Shape

`java/util/Properties` is registered with superclass `java/util/Hashtable`. Duke also registers a minimal synthetic `Hashtable` using the existing flat map layout so bytecode sees the expected hierarchy and `instanceof Hashtable` does not diverge for this slice.

`Properties` itself does not reuse the raw `Hashtable` field layout directly. Its heap layout is:

- `fields[0]`: inherited local entry count
- `fields[1]`: optional defaults `Properties`
- `fields[2..]`: local key/value pairs

The `Properties` natives are registered directly on `java/util/Properties` so inherited-looking calls such as `size`, `containsKey`, `keySet`, `values`, and `entrySet` observe the `Properties` layout rather than treating the defaults slot as a map key.

## Known Divergences

- Duke is single-threaded here; `Hashtable` synchronization semantics are intentionally not modeled.
- Raw non-String `Hashtable.put` entries on a `Properties` receiver are out of scope. `setProperty` enforces String keys and values at the synthetic boundary, and `getProperty` only returns String values.
- `load(Reader)`, `store(Writer, String)`, XML load/store, and debug `list(...)` APIs remain out of scope.
- `store` emits a fixed UTC timestamp line for deterministic tests.
