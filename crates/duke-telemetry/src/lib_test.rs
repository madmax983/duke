use super::*;
#[test]
fn test_print_report() {
    let mut store = TelemetryStore::new();
    let mut out = Vec::new();
    store.print_report(&mut out).unwrap();
    let mut out2 = Vec::new();
    store.print_report_markdown(&mut out2).unwrap();
}
