pub(crate) fn one_line(details: &str) -> String {
    details.split_whitespace().collect::<Vec<&str>>().join(" ")
}
