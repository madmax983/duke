#!/bin/bash
awk '/crates\/duke-telemetry\/src\/helpers.rs/,/crates\/duke-telemetry\/src\/lib.rs/' lcov.info
