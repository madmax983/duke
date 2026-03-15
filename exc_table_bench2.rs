use std::time::Instant;

#[derive(Clone)]
struct ExceptionEntry {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: Option<String>,
}

fn main() {
    let mut table = Vec::new();
    for _ in 0..10 {
        table.push(ExceptionEntry {
            start_pc: 0,
            end_pc: 10,
            handler_pc: 5,
            catch_type: Some("java/lang/Exception".to_string()),
        });
    }

    let t0 = Instant::now();
    for _ in 0..1_000_000 {
        let cloned: Vec<_> = table.iter().map(|e| ExceptionEntry {
            start_pc: e.start_pc,
            end_pc: e.end_pc,
            handler_pc: e.handler_pc,
            catch_type: e.catch_type.clone(),
        }).collect();
        let _ = cloned;
    }
    let t1 = t0.elapsed();

    let t2 = Instant::now();
    for _ in 0..1_000_000 {
        let cloned = table.clone();
        let _ = cloned;
    }
    let t3 = t2.elapsed();

    println!("map+collect: {:?}", t1);
    println!("Vec::clone : {:?}", t3);
}
