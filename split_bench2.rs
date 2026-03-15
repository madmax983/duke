use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let mut fs = vec![10, 20, 30, 40, 50];
    let arg_count = 5;
    for _ in 0..10_000_000 {
        fs.extend_from_slice(&[10, 20, 30, 40, 50]);
        let mut args: Vec<usize> = (0..arg_count).map(|_| fs.pop().unwrap()).collect();
        args.reverse();
        let _ = args;
    }
    let t1 = t0.elapsed();

    let t2 = Instant::now();
    for _ in 0..10_000_000 {
        fs.extend_from_slice(&[10, 20, 30, 40, 50]);
        let start = fs.len() - arg_count;
        let args = fs.split_off(start);
        let _ = args;
    }
    let t3 = t2.elapsed();

    println!("pop+collect+rev: {:?}", t1);
    println!("split_off      : {:?}", t3);
}
