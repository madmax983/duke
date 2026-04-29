#!/bin/bash
awk '/crates\/duke-telemetry\/src\/lib.rs/,/crates\/duke-telemetry\/src\/native_boundary.rs/' lcov.info
