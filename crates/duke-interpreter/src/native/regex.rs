fn regex_java_exception(class_name: &str, message: impl Into<String>) -> Error {
    push_pending_java_exception_message(class_name, message.into());
    Error::JavaException {
        class_name: class_name.to_string(),
    }
}
fn regex_pattern_syntax_error(message: impl Into<String>) -> Error {
    regex_java_exception("java/util/regex/PatternSyntaxException", message)
}
fn regex_illegal_argument(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalArgumentException", message)
}
fn regex_illegal_state(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IllegalStateException", message)
}
fn regex_index_out_of_bounds(message: impl Into<String>) -> Error {
    regex_java_exception("java/lang/IndexOutOfBoundsException", message)
}
fn regex_split_parts(re: &regex::Regex, input: &str, limit: i32) -> Vec<String> {
    if limit > 0 {
        return re
            .splitn(input, usize::try_from(limit).unwrap_or(usize::MAX))
            .map(str::to_string)
            .collect();
    }
    let mut parts: Vec<String> = re.split(input).map(str::to_string).collect();
    if limit == 0 {
        while parts.last().is_some_and(String::is_empty) {
            parts.pop();
        }
    }
    parts
}
