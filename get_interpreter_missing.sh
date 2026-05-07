#!/bin/bash
cat crates/duke-interpreter/src/context.rs | grep -E '^pub (struct|enum|trait|type|fn) '
echo "---"
cat crates/duke-interpreter/src/registry.rs | grep -E '^pub (struct|enum|trait|type|fn) '
echo "---"
cat crates/duke-interpreter/src/threading.rs | grep -E '^pub (struct|enum|trait|type|fn) '
