#!/bin/bash
set -euo pipefail

# Prepend #![cfg(test)] to fuzz.rs as well just in case.
cd crates/duke-interpreter/src/
echo "#![cfg(test)]" > fuzz.rs.new
cat fuzz.rs >> fuzz.rs.new
mv fuzz.rs.new fuzz.rs
