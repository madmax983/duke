awk '/SF:\/app\/crates\/duke-telemetry\/src\/lib.rs/,/end_of_record/' lcov.info > lcov_lib.txt
cat lcov_lib.txt
