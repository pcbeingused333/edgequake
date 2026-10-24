//! SPEC-174 — enforcement off keeps an unlabeled row readable.
//!
//! This is the Acc-shaped gate. It does not load the medical corpus.
//! Skips unless `DATABASE_URL` points at a database that has migration 220.
#![cfg(feature = "postgres")]

use sqlx::PgPool;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.is_empty())?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn e2e_spec174_enforcement_off() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let visible: bool = sqlx::query_scalar("SELECT public.clearance_covers(NULL, NULL)")
        .fetch_one(&pool)
        .await
        .expect("migration 220");
    assert!(
        visible,
        "unset clearance GUC must leave unlabeled rows readable"
    );
}
