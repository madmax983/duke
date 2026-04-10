import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Let's remove ALL test definitions for parse_arg_descriptors_test and just insert it once.
code = re.sub(
    r"\s*#\[test\]\s*fn parse_arg_descriptors_test\(\) \{[\s\S]*?\}\n",
    "",
    code
)

append_str = r"""
    #[test]
    fn parse_arg_descriptors_test() {
        let empty_vec: Vec<String> = vec![];
        assert_eq!(parse_arg_descriptors("(I)V"), vec!["I".to_string()]);
        assert_eq!(parse_arg_descriptors("(IZB)V"), vec!["I".to_string(), "Z".to_string(), "B".to_string()]);
        assert_eq!(parse_arg_descriptors("()V"), empty_vec);
        assert_eq!(parse_arg_descriptors("(Ljava/lang/String;)V"), vec!["Ljava/lang/String;".to_string()]);
        assert_eq!(parse_arg_descriptors("([I)V"), vec!["[I".to_string()]);
        assert_eq!(parse_arg_descriptors("([Ljava/lang/String;)V"), vec!["[Ljava/lang/String;".to_string()]);
        assert_eq!(parse_arg_descriptors("(ILjava/lang/String;[I)V"), vec!["I".to_string(), "Ljava/lang/String;".to_string(), "[I".to_string()]);
        assert_eq!(parse_arg_descriptors("invalid"), empty_vec);
        assert_eq!(parse_arg_descriptors("("), empty_vec);
        assert_eq!(parse_arg_descriptors(")"), empty_vec);
        assert_eq!(parse_arg_descriptors("(I"), empty_vec);
    }
"""

code = code.replace(
    r"""    #[test]
    fn parse_arg_types_malformed() {""",
    append_str + r"""
    #[test]
    fn parse_arg_types_malformed() {"""
)

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(code)
