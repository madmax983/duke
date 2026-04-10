git checkout crates/duke-loader/src/jimage.rs
sed -i '902,$d' crates/duke-loader/src/jimage.rs
cargo test -p duke-loader
