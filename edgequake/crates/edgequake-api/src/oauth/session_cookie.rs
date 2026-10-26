//! HttpOnly browser session for `GET /oauth/authorize` (MCP OAuth popup).
//!
//! SPEC-154 keeps the SPA access token in memory and scopes `eq_refresh` to
//! `/api/v1/auth`, so the browser never sends a session to `/oauth/*`.
//! This cookie is the authorize endpoint's session: HttpOnly, `Path=/oauth`
//! only, same lifetime as the access JWT. It is not readable by page script.

use axum::http::HeaderValue;

pub const OAUTH_SESSION_COOKIE: &str = "eq_oauth_session";
pub const OAUTH_SESSION_COOKIE_PATH: &str = "/oauth";

/// `Set-Cookie` for the short-lived access JWT the authorize endpoint verifies.
pub fn set_oauth_session_cookie_header(
    token: &str,
    max_age_secs: i64,
    secure: bool,
) -> HeaderValue {
    let max_age = max_age_secs.max(0);
    let mut value = format!(
        "{OAUTH_SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path={OAUTH_SESSION_COOKIE_PATH}; Max-Age={max_age}"
    );
    if secure {
        value.push_str("; Secure");
    }
    HeaderValue::from_str(&value).unwrap_or_else(|_| {
        HeaderValue::from_static(
            "eq_oauth_session=; HttpOnly; SameSite=Lax; Path=/oauth; Max-Age=0",
        )
    })
}

/// Clear the OAuth browser session (logout).
pub fn clear_oauth_session_cookie_header(secure: bool) -> HeaderValue {
    let mut value = format!(
        "{OAUTH_SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path={OAUTH_SESSION_COOKIE_PATH}; Max-Age=0"
    );
    if secure {
        value.push_str("; Secure");
    }
    HeaderValue::from_str(&value).unwrap_or_else(|_| {
        HeaderValue::from_static(
            "eq_oauth_session=; HttpOnly; SameSite=Lax; Path=/oauth; Max-Age=0",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_session_cookie_is_httponly_and_scoped_to_oauth() {
        let header = set_oauth_session_cookie_header("eyJ.aaa.bbb", 900, true);
        let value = header.to_str().expect("header");
        assert!(value.starts_with("eq_oauth_session=eyJ.aaa.bbb;"));
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("SameSite=Lax"));
        assert!(value.contains("Path=/oauth"));
        assert!(!value.contains("Path=/;"));
        assert!(value.contains("Max-Age=900"));
        assert!(value.contains("Secure"));
    }

    #[test]
    fn clear_cookie_expires_the_oauth_path() {
        let value = clear_oauth_session_cookie_header(false)
            .to_str()
            .expect("header")
            .to_string();
        assert!(value.contains("eq_oauth_session=;"));
        assert!(value.contains("Path=/oauth"));
        assert!(value.contains("Max-Age=0"));
        assert!(!value.contains("Secure"));
    }
}
