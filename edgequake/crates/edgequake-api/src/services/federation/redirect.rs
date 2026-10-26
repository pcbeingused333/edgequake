//! Post-login redirect hygiene (SPEC-158 LAW-158-4 / EC-158-30).

/// Return `candidate` only when it is a same-origin absolute *path*; otherwise `/`.
///
/// Rejects scheme-relative (`//evil`), absolute URLs, backslash tricks, control characters and
/// anything not starting with a single `/`. A query string may carry an OAuth `redirect_uri`
/// (`https://…`); that stays on this origin until `/oauth/authorize` checks the client, so
/// `://` is judged on the path only.
pub fn safe_redirect_path(candidate: Option<&str>) -> String {
    let Some(raw) = candidate.map(str::trim).filter(|path| !path.is_empty()) else {
        return "/".to_string();
    };
    let path_only = raw.split(['?', '#']).next().unwrap_or(raw);
    let path_lower = path_only.to_ascii_lowercase();
    let bad = !path_only.starts_with('/')
        || path_only.starts_with("//")
        || path_only.contains('\\')
        || path_only.contains("://")
        || path_only.contains("..")
        || raw.chars().any(char::is_control)
        // `/\evil` and `/%5Cevil` normalise to `//evil` in some browsers.
        || path_lower.starts_with("/%5c")
        || path_lower.starts_with("/%2f");
    if bad {
        "/".to_string()
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_plain_paths() {
        assert_eq!(safe_redirect_path(Some("/documents?x=1")), "/documents?x=1");
        assert_eq!(
            safe_redirect_path(Some(
                "/oauth/authorize?redirect_uri=https://client.example/cb&state=s1"
            )),
            "/oauth/authorize?redirect_uri=https://client.example/cb&state=s1"
        );
    }

    #[test]
    fn rejects_open_redirect_vectors() {
        for bad in [
            "//evil.test",
            "https://evil.test",
            "/\\evil.test",
            "javascript:alert(1)",
            "/%5Cevil.test",
            "/%2fevil.test",
            "evil",
            "",
            "/a\nb",
            "/x://y",
        ] {
            assert_eq!(safe_redirect_path(Some(bad)), "/", "{bad:?}");
        }
        assert_eq!(safe_redirect_path(None), "/");
    }
}
