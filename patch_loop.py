import sys

loop_file = 'crates/duke-bytecode/src/loop_detection.rs'

with open(loop_file, 'r') as f:
    content = f.read()

content = content.replace("assert_eq!(back_edges[0].source_pc, 7); // The block ending at Goto(2) starts at 7", "assert_eq!(back_edges[0].source_pc, 7); // The block ending at Goto(2) starts at 7")
content = content.replace("assert_eq!(edges[0].source_pc, 7); // The block starting at 7 ends with Goto(2)", "assert_eq!(edges[0].source_pc, 7); // The block starting at 7 ends with Goto(2)")
content = content.replace("assert_eq!(edges[0].source_pc, 1);", "assert_eq!(edges[0].source_pc, 1);")

with open(loop_file, 'w') as f:
    f.write(content)
