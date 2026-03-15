use std::collections::HashMap;
use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    for _ in 0..1_000_000 {
        let mut map = HashMap::new();
        map.insert(0, 0);
    }
    let t1 = t0.elapsed();
    println!("HashMap::new(): {:?}", t1);
}
