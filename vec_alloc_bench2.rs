use std::time::Instant;

fn main() {
    let mut frame_stack = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let arg_count = 5;

    // Original path used in duke-interpreter: mapping frame.pop() into Vec
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        // Reset state
        let mut fs = vec![10, 20, 30, 40, 50];

        let mut args: Vec<usize> = (0..arg_count).map(|_| fs.pop().unwrap()).collect();
        args.reverse();
        let _ = args;
    }
    let t1 = t0.elapsed();

    // New path: split_off or draining
    let t2 = Instant::now();
    for _ in 0..10_000_000 {
        // Reset state
        let mut fs = vec![10, 20, 30, 40, 50];

        let start = fs.len() - arg_count;
        let args = fs.split_off(start);
        let _ = args;
    }
    let t3 = t2.elapsed();

    println!("Original pop+collect: {:?}", t1);
    println!("Vec::split_off    : {:?}", t3);
}
