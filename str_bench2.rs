use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        let s = "java/lang/String".to_string();
        let _ = s;
    }
    let t1 = t0.elapsed();
    println!("to_string()          : {:?}", t1);
}
