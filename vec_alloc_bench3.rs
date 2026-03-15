use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        let mut fs = vec![10, 20, 30, 40, 50];
        let arg_count = 5;
        let mut args = vec![0; arg_count];
        for i in (0..arg_count).rev() {
            args[i] = fs.pop().unwrap();
        }
    }
    let t1 = t0.elapsed();

    println!("Vec alloc + rev loop : {:?}", t1);
}
