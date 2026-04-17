cat << 'INNER_EOF' > src_main_patch.sh
sed -i 's/fn test_extract_bbcfg_old/fn test_extract_bbcfg/g' duke/src/main.rs
INNER_EOF
bash src_main_patch.sh
cargo fmt --all
