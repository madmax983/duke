cargo check --all-targets --all-features
cargo test --bin duke
cargo clippy --bin duke --all-targets --all-features -- -D warnings
cargo fmt --all
