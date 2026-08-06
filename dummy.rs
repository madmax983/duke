fn main() {
    let instructions = vec![
        (0, duke_bytecode::Instruction::Goto(6)),
        (3, duke_bytecode::Instruction::Iconst1), // dead
        (4, duke_bytecode::Instruction::Ireturn),
        (6, duke_bytecode::Instruction::Iconst2), // target
        (7, duke_bytecode::Instruction::Ireturn),
    ];
    let blocks = duke_bytecode::build_basic_blocks(&instructions);
    println!("{:#?}", blocks);
    let path = duke_bytecode::find_shortest_path(&blocks, 0, 7);
    println!("path: {:?}", path);
}
