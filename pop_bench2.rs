use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        let mut fs = vec![10, 20, 30, 40, 50];
        let arg_count = 5;
        let mut args: Vec<usize> = (0..arg_count).map(|_| fs.pop().unwrap()).collect();
        args.reverse();
        let _ = args;
    }
    let t1 = t0.elapsed();

    let t2 = Instant::now();
    for _ in 0..10_000_000 {
        let mut fs = vec![10, 20, 30, 40, 50];
        let arg_count = 5;
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(fs.pop().unwrap());
        }
        args.reverse();
        let _ = args;
    }
    let t3 = t2.elapsed();

    let t4 = Instant::now();
    for _ in 0..10_000_000 {
        let mut fs = vec![10, 20, 30, 40, 50];
        let arg_count = 5;
        let start = fs.len() - arg_count;
        let args = fs.split_off(start);
        let _ = args;
    }
    let t5 = t4.elapsed();

    println!("map+collect+rev: {:?}", t1);
    println!("with_cap+rev   : {:?}", t3);
    println!("split_off      : {:?}", t5);
}
