//! Fleet mode versus the pointer rows (LAW-172-7).

use super::config::StoreMode;

/// What boot does with the rows it can see and the bucket probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootDecision {
    /// Process may listen. `ready` is false when `/ready` must be 503.
    Serve { ready: bool },
    /// Exit before the listener binds.
    Exit(String),
}

/// Pure decision. SQL counting lives in the postgres adapter.
pub fn boot_decision(
    mode: StoreMode,
    ref_rows: i64,
    holes: i64,
    bucket_ready: bool,
) -> BootDecision {
    match mode {
        StoreMode::Database if ref_rows > 0 => BootDecision::Exit(
            "EDGEQUAKE_ORIGINAL_STORE=database but original_object_refs is not empty. \
             Restart with EDGEQUAKE_ORIGINAL_STORE=object, or run `edgequake originals restore` \
             and clear the refs before using database mode."
                .into(),
        ),
        StoreMode::Object if holes > 0 && !bucket_ready => BootDecision::Exit(
            "object mode has originals with neither BYTEA nor a reachable object. \
             The bucket head failed. Fix the endpoint or restore the bytes before serving."
                .into(),
        ),
        StoreMode::Object if !bucket_ready || holes > 0 => BootDecision::Serve { ready: false },
        _ => BootDecision::Serve { ready: true },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_mode_with_a_ref_exits_before_listen() {
        match boot_decision(StoreMode::Database, 1, 0, true) {
            BootDecision::Exit(message) => {
                assert!(message.contains("original_object_refs"));
                assert!(message.contains("database"));
            }
            BootDecision::Serve { .. } => panic!("expected exit"),
        }
    }

    #[test]
    fn object_mode_with_a_down_bucket_stays_up_but_not_ready() {
        assert_eq!(
            boot_decision(StoreMode::Object, 0, 0, false),
            BootDecision::Serve { ready: false }
        );
    }

    #[test]
    fn object_mode_hole_and_down_bucket_exits() {
        assert!(matches!(
            boot_decision(StoreMode::Object, 0, 2, false),
            BootDecision::Exit(_)
        ));
    }
}
