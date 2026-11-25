pub fn parse(exit_code: Option<i32>) -> Option<String> {
    exit_code.map(|code| code.to_string())
}
