//! Shared byte caps for MCP blob and image responses (SPEC-161 / SPEC-162).

/// Hard cap per download call (SPEC-162 R10). Env may only lower this value.
pub const DOWNLOAD_HARD_CAP: usize = 65_536;

pub fn blob_max_bytes() -> usize {
    std::env::var("EDGEQUAKE_MCP_BLOB_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|n: &usize| *n > 0)
        .map(|n| n.min(DOWNLOAD_HARD_CAP))
        .unwrap_or(DOWNLOAD_HARD_CAP)
}

/// Signed URL lifetime cap (SPEC-173). The env may only lower it.
pub const UPLOAD_URL_TTL_CAP: u64 = 900;

pub fn upload_ttl_secs() -> u64 {
    cap_upload_ttl(
        std::env::var("EDGEQUAKE_MCP_UPLOAD_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok()),
    )
}

pub fn cap_upload_ttl(raw: Option<u64>) -> u64 {
    raw.filter(|n| *n > 0)
        .map(|n| n.min(UPLOAD_URL_TTL_CAP))
        .unwrap_or(UPLOAD_URL_TTL_CAP)
}

pub const EXTRA_CONTENT_KEY: &str = "_mcp_extra_content";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_env_cannot_raise_the_cap() {
        assert_eq!(cap_upload_ttl(None), 900);
        assert_eq!(cap_upload_ttl(Some(0)), 900);
        assert_eq!(cap_upload_ttl(Some(30)), 30);
        assert_eq!(cap_upload_ttl(Some(5_000)), 900);
    }
}
