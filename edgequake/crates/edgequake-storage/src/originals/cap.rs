//! One upload cap for `validate_pdf_data` and `ResourceBudget` (F-172-21).

/// Default upload ceiling (SPEC-083, 50 MiB).
pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

/// Env override. Floor is 1 MiB. Ceiling is 1 GiB.
pub const MAX_UPLOAD_BYTES_ENV: &str = "EDGEQUAKE_MAX_UPLOAD_BYTES";

const UPLOAD_FLOOR: usize = 1024 * 1024;
const UPLOAD_CEILING: usize = 1024 * 1024 * 1024;

/// Resolved upload cap. Unset env keeps [`MAX_UPLOAD_BYTES`].
pub fn max_upload_bytes() -> usize {
    let parsed = std::env::var(MAX_UPLOAD_BYTES_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok());
    bound_upload_cap(parsed)
}

fn bound_upload_cap(override_bytes: Option<usize>) -> usize {
    match override_bytes {
        Some(n) => n.clamp(UPLOAD_FLOOR, UPLOAD_CEILING),
        None => MAX_UPLOAD_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cap_is_50_mib() {
        // This test does not set the env var. A surrounding test that does
        // would change the number; the product default is the constant.
        if std::env::var(MAX_UPLOAD_BYTES_ENV).is_err() {
            assert_eq!(max_upload_bytes(), MAX_UPLOAD_BYTES);
        }
    }

    #[test]
    fn upload_cap_ceiling_is_one_gibibyte() {
        assert_eq!(bound_upload_cap(None), MAX_UPLOAD_BYTES);
        assert_eq!(bound_upload_cap(Some(1)), 1024 * 1024);
        assert_eq!(bound_upload_cap(Some(2 * 1024 * 1024)), 2 * 1024 * 1024);
        assert_eq!(bound_upload_cap(Some(usize::MAX)), 1024 * 1024 * 1024);
    }
}
