//! SPEC-175 W2 — migration 221 constraints for `model_catalog_policy`.
//!
//! Skips unless `DATABASE_URL` points at a database that has migration 221.
//! Every write runs inside one transaction that is rolled back, so the database is
//! left unchanged.
#![cfg(feature = "postgres")]

use sqlx::PgPool;
use uuid::Uuid;

const DEFAULT_TENANT: &str = "00000000-0000-0000-0000-000000000002";
const DEFAULT_WORKSPACE: &str = "00000000-0000-0000-0000-000000000003";
const SYSTEM_TENANT: &str = "00000000-0000-0000-0000-000000000000";

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.is_empty())?;
    PgPool::connect(&url).await.ok()
}

async fn insert_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: Uuid,
    workspace: Option<Uuid>,
    usage: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO public.model_catalog_policy (tenant_id, workspace_id, provider, model, usage) \
         VALUES ($1, $2, 'openai', 'gpt-5-nano', $3)",
    )
    .bind(tenant)
    .bind(workspace)
    .bind(usage)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Insert one row in a transaction that always rolls back. A failed statement aborts
/// its transaction, so every attempt gets a fresh one.
async fn attempt(
    pool: &PgPool,
    tenant: Uuid,
    workspace: Option<Uuid>,
    usage: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let result = insert_row(&mut tx, tenant, workspace, usage).await;
    tx.rollback().await?;
    result
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|db| db.code().map(|code| code.into_owned()))
}

#[tokio::test]
async fn e2e_spec175_model_policy_constraints() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let applied: bool =
        sqlx::query_scalar("SELECT to_regclass('public.model_catalog_policy') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("catalog probe");
    if !applied {
        eprintln!("SKIP: migration 221 not applied");
        return;
    }

    let tenant: Uuid = DEFAULT_TENANT.parse().unwrap();
    let workspace: Uuid = DEFAULT_WORKSPACE.parse().unwrap();
    let system_tenant: Uuid = SYSTEM_TENANT.parse().unwrap();

    attempt(&pool, tenant, None, "query")
        .await
        .expect("ceiling row is accepted");

    // Two ceiling rows for one pair must collide in the same transaction.
    let mut tx = pool.begin().await.expect("begin");
    insert_row(&mut tx, tenant, None, "query")
        .await
        .expect("first ceiling row in transaction");
    let ceiling_twice = insert_row(&mut tx, tenant, None, "query").await;
    assert_eq!(
        sqlstate(&ceiling_twice.unwrap_err()).as_deref(),
        Some("23505")
    );
    tx.rollback().await.expect("rollback");

    let unknown_usage = attempt(&pool, tenant, None, "chat").await;
    assert_eq!(
        sqlstate(&unknown_usage.unwrap_err()).as_deref(),
        Some("23514")
    );

    attempt(&pool, tenant, Some(workspace), "query")
        .await
        .expect("narrowing row in its own tenant is accepted");

    let cross_tenant = attempt(&pool, system_tenant, Some(workspace), "query").await;
    assert_eq!(
        sqlstate(&cross_tenant.unwrap_err()).as_deref(),
        Some("23503"),
        "a narrowing row must not name a workspace of another tenant"
    );
}
