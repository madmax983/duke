import re
with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    content = f.read()

search = """        let handle = std::thread::spawn(|| {
            let result = std::panic::catch_unwind(|| {
                crate::tests::run_bootstrap_with_output(
                    "JoinSelf.class",
                    "main",
                    "([Ljava/lang/String;)V",
                )
            });
            result
        });"""

replace = """        let handle = std::thread::spawn(|| {
            std::panic::catch_unwind(|| {
                crate::tests::run_bootstrap_with_output(
                    "JoinSelf.class",
                    "main",
                    "([Ljava/lang/String;)V",
                )
            })
        });"""

with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(content.replace(search, replace))
