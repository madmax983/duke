use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        let mut s = String::new();
        s.push_str("java/lang/String");
        let _ = s;
    }
    let t1 = t0.elapsed();

    let t2 = Instant::now();
    let s_const = "java/lang/String".to_string();
    for _ in 0..10_000_000 {
        let s = s_const.clone();
        let _ = s;
    }
    let t3 = t2.elapsed();

    println!("String::new().push() : {:?}", t1);
    println!("String::clone()      : {:?}", t3);
}
