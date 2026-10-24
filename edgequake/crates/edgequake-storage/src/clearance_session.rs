//! Request-scoped clearance for SQL.
//!
//! `bind_request` builds one [`ClearanceSession`] and the HTTP middleware
//! enters it for the rest of the request. Storage reads that task-local
//! value when it opens a transaction. Handlers do not pass the session
//! through every signature, and they do not compile rules themselves.

use std::future::Future;
use uuid::Uuid;

tokio::task_local! {
    static CURRENT: ClearanceSession;
}

/// Compiled subject, already reduced to the values PostgreSQL needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClearanceSession {
    pub enforced: bool,
    pub break_glass: bool,
    /// Ingest and other workers. Not a user skip and not audited as one.
    pub system: bool,
    pub steward: bool,
    pub rank: i16,
    pub compartments: Vec<Uuid>,
    pub allow_empty: bool,
    pub user_id: Option<Uuid>,
    pub policy_epoch: i64,
    pub fingerprint: String,
}

impl ClearanceSession {
    /// Enforcement off. Lists and query keep the pre-174 path.
    pub fn off() -> Self {
        Self {
            enforced: false,
            break_glass: false,
            system: false,
            steward: false,
            rank: 0,
            compartments: Vec::new(),
            allow_empty: true,
            user_id: None,
            policy_epoch: 0,
            fingerprint: "off".to_string(),
        }
    }

    pub fn current() -> Option<Self> {
        CURRENT.try_with(|session| session.clone()).ok()
    }

    /// Extra predicate for document SQL. Empty unless this request enforces
    /// clearance and is not break-glass. The caller must [`apply_session`]
    /// on the same transaction first.
    pub fn predicate_sql(&self) -> &'static str {
        if self.enforced && !self.break_glass && !self.system {
            " AND (public.clearance_covers(classification_rank, compartment_ids) OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))"
        } else {
            ""
        }
    }

    pub fn compartments_literal(&self) -> String {
        if self.compartments.is_empty() {
            return String::new();
        }
        let body = self
            .compartments
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{body}}}")
    }
}

/// Run `future` with `session` visible to storage on this task.
pub async fn scope<F>(session: ClearanceSession, future: F) -> F::Output
where
    F: Future,
{
    CURRENT.scope(session, future).await
}

/// Predicate for the session on this task, or empty when the caller did not bind one.
pub fn current_predicate_sql() -> &'static str {
    ClearanceSession::current()
        .map(|session| session.predicate_sql())
        .unwrap_or("")
}
