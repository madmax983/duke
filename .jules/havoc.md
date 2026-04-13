**[Title]
**Tangle:** Tried to crash duke_classfile, duke_loader, duke_gc, and duke_interpreter with garbage data and fuzz tests using proptest.
**Blueprint:** Found no panics. The codebase correctly uses `try_from`, `min()`, `checked_add`, `unwrap_or`, and proper Error returning instead of unwrapping blindly on untrusted inputs. I'll summarize these findings and submit.
## Zip Bomb Vulnerability in ZipLoader
**The Target:** `ZipLoader::read_entry_info` decompression (`flate2::read::DeflateDecoder`)

**The Attack:** An attacker constructs a ZIP file with heavily compressed zero-bytes (e.g., 10MB compressed) but fakes the `uncompressed_size` header to `0xFFFFFFFF` (4GB). The loader would attempt to decompress the entire archive into memory. Due to the fake header, `flate2::read::DeflateDecoder` might try to read huge amounts of data and crash with OOM or exhaust memory without bound.

**The Fix:** Enforce a hard maximum memory usage limit (`MAX_SIZE = 256MB`) during zip decompression. Also, chain the `std::io::Read::take(MAX_SIZE)` adapter to the `DeflateDecoder` so that it physically cannot read more bytes than the threshold, stopping Zip Bombs immediately.
