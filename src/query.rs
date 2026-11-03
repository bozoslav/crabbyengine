//! Reading a page's query string (`?name=value&...`), the web build's
//! stand-in for environment variables.

/// The value of `name` in `search`: `Some("")` when it's there without a
/// value, `None` when it isn't there.
pub(crate) fn value(search: &str, name: &str) -> Option<String> {
    search.trim_start_matches('?').split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (key == name).then(|| value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn finds_values_flags_and_nothing() {
        assert_eq!(value("?inspector=open&x=1", "inspector").as_deref(), Some("open"));
        assert_eq!(value("?x=1&inspector", "inspector").as_deref(), Some(""));
        assert_eq!(value("?inspectorx=1", "inspector"), None);
        assert_eq!(value("", "inspector"), None);
    }
}
