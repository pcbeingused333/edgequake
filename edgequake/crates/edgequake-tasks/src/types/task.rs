//! Core Task struct and lifecycle management.
//!
//! Contains the Task struct (the main unit of work), its lifecycle
//! methods (mark_processing, mark_success, mark_failed, etc.),
//! circuit breaker logic, and the track ID generator.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::failure::TaskFailureInfo;
use super::progress::{ChunkProgress, TaskProgress};
use super::status::{TaskStatus, TaskType};

/// A background task representing a unit of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique track ID: {type}-{uuid}
    pub track_id: String,

    /// Tenant ID for multi-tenancy isolation
    pub tenant_id: Uuid,

    /// Workspace ID for workspace-level isolation
    pub workspace_id: Uuid,

    /// Type of task
    pub task_type: TaskType,

    /// Current status
    pub status: TaskStatus,

    /// When task was created
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// When processing started
    pub started_at: Option<DateTime<Utc>>,

    /// When task completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if failed (kept for backward compatibility)
    pub error_message: Option<String>,

    /// Detailed error information (Phase 1 enhancement)
    pub error: Option<TaskFailureInfo>,

    /// Number of retry attempts
    pub retry_count: i32,

    /// Maximum retries allowed
    pub max_retries: i32,

    /// Number of consecutive timeout failures (for circuit breaker).
    ///
    /// @implements CIRCUIT_BREAKER: Track consecutive timeouts
    ///
    /// WHY: Timeouts indicate document is too large or LLM is overloaded.
    /// After 3 consecutive timeouts, we should fail permanently rather than
    /// waste resources retrying. Non-timeout failures (network errors, etc.)
    /// don't count toward this limit because they may resolve on retry.
    ///
    /// Reset to 0 on:
    /// - Successful processing
    /// - Non-timeout failure (network error, validation error, etc.)
    ///
    /// Increment on:
    /// - LLM timeout error
    /// - Embedding timeout error
    ///
    /// Circuit breaker trips when: consecutive_timeout_failures >= 3
    pub consecutive_timeout_failures: i32,

    /// Whether circuit breaker has permanently failed this task.
    ///
    /// @implements CIRCUIT_BREAKER: Permanent failure flag
    ///
    /// WHY: Prevents infinite retries on documents that consistently timeout.
    /// Once tripped, task is marked Failed and won't be retried again.
    pub circuit_breaker_tripped: bool,

    /// Task-specific payload
    pub task_data: serde_json::Value,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,

    /// Progress information
    pub progress: Option<TaskProgress>,

    /// Result data (on success)
    pub result: Option<serde_json::Value>,

    /// Worker id holding the processing lease (SPEC-057 P1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_owner: Option<String>,

    /// CAS token for refresh_lease / release_claim (SPEC-057 P1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_token: Option<Uuid>,

    /// When the processing lease expires (SPEC-057 P1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at: Option<DateTime<Utc>>,

    /// Claim-invisible fairness park until this instant (SPEC-057 INV-06).
    ///
    /// `None` or past → eligible for `claim_next`. Set by `mark_fairness_hold`;
    /// cleared on park wake. TTL expiry is crash-safety for stranded holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fairness_hold_until: Option<DateTime<Utc>>,
}

impl Task {
    /// PDF id from convert (`task_data.pdf_id`) or ingest (`metadata.pdf_id`).
    ///
    /// SPEC-057 P2: Insert follow-on tasks carry `pdf_id` under metadata so
    /// single-flight / cancel chain can match Convert + Ingest for the same PDF.
    pub fn pdf_id(&self) -> Option<Uuid> {
        self.task_data
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| {
                self.task_data
                    .get("metadata")
                    .and_then(|m| m.get("pdf_id"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
    }

    /// Document id from payload / metadata (issue #384 identity SSOT).
    ///
    /// Order matches the historical API extractor so PDF reprocess
    /// (`existing_document_id`) wins over text insert (`metadata.document_id`).
    /// Trimmed empty strings are treated as absent.
    pub fn document_id(&self) -> Option<String> {
        self.task_data
            .get("existing_document_id")
            .and_then(|v| v.as_str())
            .or_else(|| self.task_data.get("document_id").and_then(|v| v.as_str()))
            .or_else(|| {
                self.task_data
                    .get("metadata")
                    .and_then(|m| m.get("document_id"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                self.metadata
                    .as_ref()
                    .and_then(|m| m.get("document_id"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                self.metadata
                    .as_ref()
                    .and_then(|m| m.get("existing_document_id"))
                    .and_then(|v| v.as_str())
            })
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// Create a new task
    pub fn new(
        tenant_id: Uuid,
        workspace_id: Uuid,
        task_type: TaskType,
        task_data: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        let track_id = generate_track_id(task_type);

        Self {
            track_id,
            tenant_id,
            workspace_id,
            task_type,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data,
            metadata: None,
            progress: None,
            result: None,
            lease_owner: None,
            lease_token: None,
            lease_expires_at: None,
            fairness_hold_until: None,
        }
    }

    /// Clear lease fields (terminal status or release_claim).
    pub fn clear_lease(&mut self) {
        self.lease_owner = None;
        self.lease_token = None;
        self.lease_expires_at = None;
    }

    /// True when a Processing lease is missing or past expiry (claimable / orphan).
    pub fn lease_is_expired(&self, now: DateTime<Utc>) -> bool {
        match self.lease_expires_at {
            None => true,
            Some(exp) => exp <= now,
        }
    }

    /// SPEC-091 QW0 (LAW-Q2): every status mutation is a state-machine event.
    ///
    /// Applies `event` via the single transition table
    /// ([`crate::state_machine::transition`]); an illegal event leaves the task
    /// unchanged and returns the error so callers can degrade loudly instead of
    /// occupying an impossible state.
    pub fn apply_event(
        &mut self,
        event: crate::state_machine::TaskEvent,
    ) -> Result<TaskStatus, crate::state_machine::TransitionError> {
        let next = crate::state_machine::transition(Some(self.status), event)?;
        self.status = next;
        self.updated_at = Utc::now();
        Ok(next)
    }

    /// True while an active fairness hold blocks claim (SPEC-057 INV-06).
    pub fn is_fairness_held(&self, now: DateTime<Utc>) -> bool {
        match self.fairness_hold_until {
            None => false,
            Some(until) => until > now,
        }
    }

    /// Mark task as processing (state machine: `Claim`).
    pub fn mark_processing(&mut self) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Claim)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "mark_processing ignored: illegal Claim transition"
            );
            return;
        }
        self.started_at = Some(Utc::now());
    }

    /// Mark task as completed successfully (state machine: `Complete`).
    ///
    /// Returns `false` and leaves the task unchanged when the transition is
    /// illegal for the current status (e.g. X-29: Cancelled ↛ Success).
    pub fn mark_success(&mut self, result: serde_json::Value) -> bool {
        if self
            .apply_event(crate::state_machine::TaskEvent::Complete)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "mark_success ignored: illegal Complete transition"
            );
            return false;
        }
        self.completed_at = Some(Utc::now());
        self.result = Some(result);
        self.error = None;
        self.error_message = None;
        // Reset timeout counter on success
        self.consecutive_timeout_failures = 0;
        self.clear_lease();
        true
    }

    /// Mark task as failed with simple error message (state machine: `Fail`).
    pub fn mark_failed(&mut self, error: String) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Fail)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "mark_failed ignored: illegal Fail transition"
            );
            return;
        }
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.clone());
        self.clear_lease();
        self.retry_count += 1;

        // X-30: typed timeout — only structured markers trip the breaker
        // (not business text that happens to contain the word "timeout").
        let is_typed_timeout = crate::is_typed_timeout_message(&error);
        if is_typed_timeout {
            let made_progress = error.contains("[vision_progress=1]");
            if !made_progress {
                self.consecutive_timeout_failures += 1;
                self.check_circuit_breaker();
            }
        } else {
            self.consecutive_timeout_failures = 0;
        }
    }

    /// Mark task as failed with detailed error information (state machine: `Fail`).
    pub fn mark_failed_with_details(&mut self, error: TaskFailureInfo) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Fail)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "mark_failed_with_details ignored: illegal Fail transition"
            );
            return;
        }
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.message.clone());
        self.clear_lease();

        // Set error FIRST so check_circuit_breaker can modify it
        self.error = Some(error.clone());

        // Track consecutive timeouts for circuit breaker.
        // Progress-aware: a timeout after real progress (checkpointed pages)
        // must NOT advance the breaker — only no-progress hangs trip it.
        if error.is_timeout() {
            if error.made_progress {
                // Leave consecutive_timeout_failures unchanged.
            } else {
                self.consecutive_timeout_failures += 1;
                self.check_circuit_breaker(); // Modifies self.error if circuit breaker trips
            }
        } else {
            // Non-timeout failures reset the counter
            self.consecutive_timeout_failures = 0;
        }

        self.retry_count += 1;
    }

    /// Check circuit breaker and trip if threshold exceeded.
    ///
    /// @implements CIRCUIT_BREAKER: Threshold check
    ///
    /// WHY: After 3 consecutive timeouts, permanently fail the task.
    /// Prevents wasting resources on documents that consistently timeout.
    fn check_circuit_breaker(&mut self) {
        const CIRCUIT_BREAKER_THRESHOLD: i32 = 3;

        if self.consecutive_timeout_failures >= CIRCUIT_BREAKER_THRESHOLD {
            self.circuit_breaker_tripped = true;

            // Enhance error message to indicate circuit breaker tripped
            if let Some(ref mut error) = self.error {
                error.message = format!(
                    "Circuit breaker tripped: {} consecutive timeouts. Task permanently failed.",
                    self.consecutive_timeout_failures
                );
                error.retryable = false;
            } else {
                self.error_message = Some(format!(
                    "Circuit breaker tripped after {} consecutive timeouts with no progress. \
                    Document may be blocked on a hung vision provider or a single stuck page. \
                    Suggestions: 1) Check provider health / API quotas, \
                    2) Raise EDGEQUAKE_VISION_STALL_TIMEOUT_SECS if pages are legitimately slow, \
                    3) Split the document, 4) Reprocess after fixing the provider.",
                    self.consecutive_timeout_failures
                ));
            }
        }
    }

    /// Mark task as cancelled (state machine: `Cancel`).
    pub fn mark_cancelled(&mut self) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Cancel)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "mark_cancelled ignored: illegal Cancel transition"
            );
            return;
        }
        self.completed_at = Some(Utc::now());
        self.clear_lease();
    }

    /// Requeue for automatic retry (state machine: `RetryRequeue`, failed → pending).
    ///
    /// SSOT for the worker retry path: the claim channel is wake-only, so a
    /// retryable task must be persisted as Pending again (SPEC-057 P1). Error
    /// details are retained for observability; lease + completion markers reset.
    pub fn requeue_for_retry(&mut self) {
        if self
            .apply_event(crate::state_machine::TaskEvent::RetryRequeue)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "requeue_for_retry ignored: illegal RetryRequeue transition"
            );
            return;
        }
        self.completed_at = None;
        self.clear_lease();
    }

    /// Recover an orphaned Processing task back to Pending after restart
    /// (state machine: `LeaseLost`, processing → pending).
    pub fn recover_to_pending(&mut self, note: String) {
        if self
            .apply_event(crate::state_machine::TaskEvent::LeaseLost)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "recover_to_pending ignored: illegal LeaseLost transition"
            );
            return;
        }
        self.clear_lease();
        self.started_at = None;
        self.error_message = Some(note);
    }

    /// Mark an orphaned Processing task failed for manual resume when
    /// auto-resume is disabled (state machine: `Fail`, processing → failed).
    ///
    /// Unlike [`Task::mark_failed`], no retry bookkeeping: the attempt was
    /// interrupted by the restart, not by a processing error — retry_count and
    /// the timeout breaker are untouched, and `completed_at` stays unset.
    pub fn fail_orphaned(&mut self, message: String) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Fail)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "fail_orphaned ignored: illegal Fail transition"
            );
            return;
        }
        self.clear_lease();
        self.error_message = Some(message);
    }

    /// Reflect a released claim on the in-memory copy (state machine:
    /// `Release`, processing → pending). The DB row is updated by
    /// `release_claim`; this keeps the worker's copy consistent for the
    /// fairness-park re-wake path.
    pub fn release_to_pending(&mut self) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Release)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "release_to_pending ignored: illegal Release transition"
            );
            return;
        }
        self.clear_lease();
    }

    /// Auto-close an orphaned Processing task whose document already reached a
    /// terminal-success state (state machine: `Complete`, processing → indexed).
    /// The close note rides in `error_message` for operator visibility.
    pub fn close_orphaned_indexed(&mut self, note: String) {
        if self
            .apply_event(crate::state_machine::TaskEvent::Complete)
            .is_err()
        {
            tracing::warn!(
                track_id = %self.track_id,
                status = ?self.status,
                "close_orphaned_indexed ignored: illegal Complete transition"
            );
            return;
        }
        self.clear_lease();
        self.error_message = Some(note);
    }

    /// Update task progress
    pub fn update_progress(&mut self, current_step: String, total_steps: u32, percent: u8) {
        self.progress = Some(TaskProgress {
            current_step,
            total_steps,
            percent_complete: percent.min(100),
            chunk_progress: None,
        });
        self.updated_at = Utc::now();
    }

    /// Update task progress with chunk-level tracking
    pub fn update_progress_with_chunks(
        &mut self,
        current_step: String,
        total_steps: u32,
        percent: u8,
        chunk_progress: ChunkProgress,
    ) {
        self.progress = Some(TaskProgress {
            current_step,
            total_steps,
            percent_complete: percent.min(100),
            chunk_progress: Some(chunk_progress),
        });
        self.updated_at = Utc::now();
    }

    /// Check if task can be retried
    ///
    /// @implements CIRCUIT_BREAKER: Retry eligibility check
    ///
    /// Returns false if:
    /// - Circuit breaker is tripped (3+ consecutive timeouts)
    /// - Retry limit exceeded
    /// - Error marked as not retryable
    /// - Task is not in Failed status
    pub fn can_retry(&self) -> bool {
        // Circuit breaker prevents retries
        if self.circuit_breaker_tripped {
            return false;
        }

        let is_retryable = self.error.as_ref().map(|e| e.retryable).unwrap_or(true);
        self.status == TaskStatus::Failed && self.retry_count < self.max_retries && is_retryable
    }

    /// Check if task is terminal (completed or permanently failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, TaskStatus::Indexed | TaskStatus::Cancelled)
            || (self.status == TaskStatus::Failed && !self.can_retry())
    }
}

/// Generate a track ID for a task
pub fn generate_track_id(task_type: TaskType) -> String {
    let uuid = Uuid::new_v4();
    format!("{}-{}", task_type, uuid)
}
