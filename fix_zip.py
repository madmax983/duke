import os

file_path = "crates/duke-loader/src/zip.rs"

with open(file_path, "r") as f:
    content = f.read()

search = """                let decoder = flate2::read::DeflateDecoder::new(compressed);
                let cap = info.uncompressed_size as usize;
                // Hard limit on decompression memory usage (256MB)
                const MAX_SIZE: u64 = 256 * 1024 * 1024;

                if info.uncompressed_size as u64 > MAX_SIZE {"""

replace = """                // Hard limit on decompression memory usage (256MB)
                const MAX_SIZE: u64 = 256 * 1024 * 1024;
                let decoder = flate2::read::DeflateDecoder::new(compressed);
                let cap = info.uncompressed_size as usize;

                if info.uncompressed_size > MAX_SIZE {"""

if search in content:
    content = content.replace(search, replace)
    with open(file_path, "w") as f:
        f.write(content)
    print("Replaced successfully.")
else:
    print("Could not find the target string.")
