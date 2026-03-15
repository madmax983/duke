use std::time::Instant;

fn main() {
    let arg_count = 5;

    // original: (0..arg_count).map(|_| ...).collect::<Vec<_>>().reverse()
    let t0 = Instant::now();
    for _ in 0..10_000_000 {
        let mut vec: Vec<usize> = (0..arg_count).map(|i| i).collect();
        vec.reverse();
        let _ = vec;
    }
    let t1 = t0.elapsed();

    // opt 1: Vec::with_capacity + push
    let t2 = Instant::now();
    for _ in 0..10_000_000 {
        let mut vec = Vec::with_capacity(arg_count);
        for i in 0..arg_count {
            vec.push(i);
        }
        vec.reverse();
        let _ = vec;
    }
    let t3 = t2.elapsed();

    // opt 2: manual reverse into a pre-sized vec
    let t4 = Instant::now();
    for _ in 0..10_000_000 {
        let mut vec = vec![0; arg_count];
        for i in (0..arg_count).rev() {
            vec[i] = i;
        }
        let _ = vec;
    }
    let t5 = t4.elapsed();

    println!("Original: {:?}", t1);
    println!("with_cap: {:?}", t3);
    println!("pre_size: {:?}", t5);
}
