#![no_main]

use libfuzzer_sys::fuzz_target;
use duke_telemetry::TelemetryStore;
use std::io::sink;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", s, s, 10, 100);
        let mut writer = sink();
        let _ = store.print_report(&mut writer);
    }
});
