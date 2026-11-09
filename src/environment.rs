//! Environment settings: the Ponko spelling wins, including empty or invalid values.
use std::ffi::OsString;

fn lookup(name: &str, mut get: impl FnMut(&str) -> Option<OsString>) -> Option<OsString> {
    get(name).or_else(|| {
        name.strip_prefix("PONKO_")
            .and_then(|suffix| get(&format!("CRABBY_{suffix}")))
    })
}

pub(crate) fn var_os(name: &str) -> Option<OsString> {
    lookup(name, |key| std::env::var_os(key))
}

pub(crate) fn var(name: &str) -> Result<String, std::env::VarError> {
    var_os(name)
        .ok_or(std::env::VarError::NotPresent)?
        .into_string()
        .map_err(std::env::VarError::NotUnicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_wins_even_when_empty_or_invalid() {
        for primary in ["", "0", "invalid", "1"] {
            assert_eq!(
                lookup("PONKO_CONTROL", |key| Some(
                    if key == "PONKO_CONTROL" { primary } else { "1" }.into()
                )),
                Some(primary.into())
            );
        }
    }

    #[test]
    fn legacy_is_used_only_when_primary_is_absent() {
        assert_eq!(
            lookup("PONKO_CONTROL", |key| (key == "CRABBY_CONTROL").then(|| "1".into())),
            Some("1".into())
        );
        assert_eq!(lookup("PONKO_CONTROL", |_| None), None);
        assert_eq!(
            lookup("HOME", |key| (key == "CRABBY_HOME").then(|| "wrong".into())),
            None
        );
    }
}
