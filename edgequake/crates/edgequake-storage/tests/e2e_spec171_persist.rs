//! SPEC-171 live checks: ingest `lock_timeout` survives pool release, and
//! one workspace's merge lock does not overlap.
//!
//! Skips when no database is configured. Run:
//!   cargo test -p edgequake-storage --features postgres --test e2e_spec171_persist

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::{
    apply_ingest_write_guards, log_activity_snapshot, with_session_hygiene_labeled,
    WORKSPACE_MERGE_LOCK_SQL,
};
use postgres_test_config::{require_or_skip_postgres, seed_tenant_and_user};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

async fn lock_timeout_env() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    LOCK.lock().await
}

struct RestoreLockTimeout;
impl Drop for RestoreLockTimeout {
    fn drop(&mut self) {
        std::env::remove_var("EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS");
    }
}

#[tokio::test]
async fn t171_26_lock_timeout_survives_pool_release() {
    let _env = lock_timeout_env().await;
    let Some(cfg) = require_or_skip_postgres("spec171_lock") else {
        return;
    };
    let pool =
        with_session_hygiene_labeled(PgPoolOptions::new().max_connections(4), "edgequake:ingest")
            .connect_with(cfg.connect_options())
            .await
            .expect("pool");
    let first = current_lock_timeout(&pool).await;
    let second = current_lock_timeout(&pool).await;
    assert!(
        first == "30s" || first == "30000ms",
        "first checkout lock_timeout={first}"
    );
    assert_eq!(first, second, "RESET ALL must re-pin lock_timeout");
    pool.close().await;
}

#[tokio::test]
async fn t171_28_held_row_lock_fails_inside_lock_timeout_then_retries() {
    let _env = lock_timeout_env().await;
    let _restore = RestoreLockTimeout;
    std::env::set_var("EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS", "2s");
    let Some(cfg) = require_or_skip_postgres("spec171_row") else {
        return;
    };
    let pool =
        with_session_hygiene_labeled(PgPoolOptions::new().max_connections(4), "edgequake:ingest")
            .connect_with(cfg.connect_options())
            .await
            .expect("pool");
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let document_id = Uuid::new_v4();
    seed_tenant_and_user(&pool, tenant_id, Uuid::new_v4())
        .await
        .expect("tenant");
    let slug = format!("ws_{}", &workspace_id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO workspaces (
            workspace_id, tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("SPEC171 {slug}"))
    .bind(&slug)
    .execute(&pool)
    .await
    .expect("workspace");
    sqlx::query(
        r#"
        INSERT INTO public.documents (id, tenant_id, workspace_id, content, status)
        VALUES ($1, $2, $3, '', 'processing')
        "#,
    )
    .bind(document_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("document");

    let release = Arc::new(tokio::sync::Notify::new());
    let held = Arc::new(tokio::sync::Notify::new());
    let holder_pool = pool.clone();
    let holder_release = release.clone();
    let holder_held = held.clone();
    let holder = tokio::spawn(async move {
        let mut tx = holder_pool.begin().await.expect("holder begin");
        sqlx::query("SELECT id FROM public.documents WHERE id = $1 FOR UPDATE")
            .bind(document_id)
            .fetch_one(&mut *tx)
            .await
            .expect("holder lock");
        holder_held.notify_one();
        holder_release.notified().await;
        tx.commit().await.expect("holder commit");
    });
    held.notified().await;

    let started = Instant::now();
    let mut waiter = pool.begin().await.expect("waiter begin");
    apply_ingest_write_guards(&mut waiter, Some(&workspace_id.to_string()), 30)
        .await
        .expect("guards");
    let wait_err = sqlx::query("SELECT id FROM public.documents WHERE id = $1 FOR UPDATE")
        .bind(document_id)
        .fetch_one(&mut *waiter)
        .await
        .expect_err("a held row must not wait for the 7200s task clock");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(8),
        "lock wait lasted {elapsed:?}; lock_timeout is 2s"
    );
    let code = wait_err
        .as_database_error()
        .and_then(|db| db.code().map(|code| code.to_string()));
    assert_eq!(code.as_deref(), Some("55P03"), "{wait_err}");
    waiter.rollback().await.expect("waiter rollback");

    let still_waiting: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)::bigint
        FROM pg_stat_activity
        WHERE application_name = 'edgequake:ingest'
          AND state = 'active'
          AND wait_event_type = 'Lock'
          AND query LIKE '%documents%'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("activity");
    assert_eq!(
        still_waiting, 0,
        "the cancelled waiter must not stay active"
    );

    release.notify_one();
    holder.await.expect("holder");

    let mut retry = pool.begin().await.expect("retry begin");
    apply_ingest_write_guards(&mut retry, Some(&workspace_id.to_string()), 30)
        .await
        .expect("retry guards");
    let retried: Uuid =
        sqlx::query_scalar("SELECT id FROM public.documents WHERE id = $1 FOR UPDATE")
            .bind(document_id)
            .fetch_one(&mut *retry)
            .await
            .expect("second attempt takes the row once the holder commits");
    assert_eq!(retried, document_id);
    retry.commit().await.expect("retry commit");
    pool.close().await;
}

#[tokio::test]
async fn t171_36_activity_snapshot_query_runs() {
    let Some(cfg) = require_or_skip_postgres("spec171_snap") else {
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(cfg.connect_options())
        .await
        .expect("pool");
    log_activity_snapshot(&pool)
        .await
        .expect("snapshot failure must be an error from the function, not a panic");
    pool.close().await;
}

#[tokio::test]
async fn t171_31_same_workspace_merge_lock_does_not_overlap() {
    let Some(cfg) = require_or_skip_postgres("spec171_merge") else {
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(cfg.connect_options())
        .await
        .expect("pool");
    let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let pool_a = pool.clone();
    let order_a = order.clone();
    let first = tokio::spawn(async move {
        hold_merge_lock(pool_a, "ws-same", order_a, "a").await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let pool_b = pool.clone();
    let order_b = order.clone();
    let second = tokio::spawn(async move {
        hold_merge_lock(pool_b, "ws-same", order_b, "b").await;
    });
    first.await.expect("first");
    second.await.expect("second");
    let seen = order.lock().await.clone();
    assert_eq!(
        seen,
        vec!["a-got", "a-release", "b-got", "b-release"],
        "same workspace writes must not overlap: {seen:?}"
    );

    let other = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let left = {
        let pool = pool.clone();
        let other = other.clone();
        tokio::spawn(async move { hold_merge_lock(pool, "ws-left", other, "l").await })
    };
    let right = {
        let pool = pool.clone();
        let other = other.clone();
        tokio::spawn(async move { hold_merge_lock(pool, "ws-right", other, "r").await })
    };
    left.await.expect("left");
    right.await.expect("right");
    let parallel = other.lock().await.clone();
    assert!(
        parallel
            .windows(2)
            .any(|pair| pair == ["l-got", "r-got"] || pair == ["r-got", "l-got"]),
        "different workspaces must overlap: {parallel:?}"
    );
    pool.close().await;
}

async fn current_lock_timeout(pool: &sqlx::PgPool) -> String {
    let row = sqlx::query("SELECT current_setting('lock_timeout') AS lock_timeout")
        .fetch_one(pool)
        .await
        .expect("lock_timeout");
    row.get("lock_timeout")
}

async fn hold_merge_lock(
    pool: sqlx::PgPool,
    workspace_id: &str,
    order: Arc<tokio::sync::Mutex<Vec<&'static str>>>,
    label: &'static str,
) {
    let mut tx = pool.begin().await.expect("begin");
    sqlx::query(WORKSPACE_MERGE_LOCK_SQL)
        .bind(workspace_id)
        .execute(&mut *tx)
        .await
        .expect("advisory lock");
    order.lock().await.push(match label {
        "a" => "a-got",
        "b" => "b-got",
        "l" => "l-got",
        _ => "r-got",
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    order.lock().await.push(match label {
        "a" => "a-release",
        "b" => "b-release",
        "l" => "l-release",
        _ => "r-release",
    });
    tx.commit().await.expect("commit");
}
