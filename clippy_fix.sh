#!/bin/bash
cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery -D warnings
