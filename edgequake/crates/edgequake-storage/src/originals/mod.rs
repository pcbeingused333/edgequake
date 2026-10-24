//! SPEC-172 original-byte store.
//!
//! Key format, put, ranged get, SHA-256, and delete live here. PDF storage,
//! original storage, and `edgequake originals` call this module. They do not
//! each speak HTTP to S3.

mod boot;
mod cap;
mod chunk;
mod config;
mod copy;
mod format;
mod io;
mod key;
mod ledger;
mod reaper;

#[cfg(feature = "postgres")]
mod pg;

pub use boot::{boot_decision, BootDecision};
pub use cap::{max_upload_bytes, MAX_UPLOAD_BYTES, MAX_UPLOAD_BYTES_ENV};
pub use chunk::{
    chunk_digests, chunk_matches, chunks_covering, digest_at, pack_digests, PDF_CHUNK_BYTES,
};
pub use config::{
    classify_url, open_from_env, open_settings, open_settings_with, path_style_enabled,
    settings_from_env, sse_from_env, OriginalsClient, OriginalsOpened, Settings, StoreKind,
    StoreMode,
};
pub use copy::{
    after_object_checksum, clear_owners, column_matches_ref, copy_owners, copy_owners_with,
    delete_loser, plan_text, prefer_object, restore_owners, status_text, upload_key, verify_owners,
    CopyReport, ObjectServe, RunOptions,
};
pub use format::{
    human_bytes, redact_database_url, render_clear, render_migrate, render_plan, render_status,
    suggest_next, ClearView, PlanView,
};
pub use io::{
    confirm_put, get_inclusive, inclusive_to_get_range, publish_bytes, publish_new_key, put_bytes,
    read_verified, reject_short_parts, sha256_hex, sse_headers, verify_sha256, write_strategy,
    BlobStore, FailNth, ObjectBlob, OriginalsError, PanicOnPut, PutMeta, RecordingBlob, Sse,
    StoreFault, WriteStrategy, MIN_MULTIPART_PART, SINGLE_PUT_MAX,
};
pub use key::{object_key, scoped_object_key, OwnerKind};
pub use ledger::{Claim, KindFilter, MemoryLedger, OriginalsLedger, OwnerRow, RefView, Scope};
pub use reaper::{expired_inflight, reap, InflightRow, ReapStats, INFLIGHT_TTL};

#[cfg(feature = "postgres")]
pub use pg::{
    assert_maint_role, assess_boot, asset_pointer, claim_statement_timeout, connect_maint_pool,
    enforce_boot, insert_inflight, markdown_pointer, original_pointer, pdf_pointer,
    pending_copy_count, reap_postgres, remove_delete_queue, remove_inflight, retire_previous_key,
    workspace_tenant, ObjectPointer, PgLedger, CLAIM_STATEMENT_TIMEOUT, MAINT_ROLE,
};
