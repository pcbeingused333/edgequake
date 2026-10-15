//! Cutover ledger. Plain reads follow the row, not an env override.

use sqlx::PgPool;

use crate::error::{Result, StorageError};

use super::super::{native_graph_writes_enabled, PostgresAGEGraphStorage};

impl PostgresAGEGraphStorage {
    /// Plain reads when the cutover ledger is `ready`. `EDGEQUAKE_GRAPH_READ=age` pins AGE.
    pub(in crate::adapters::postgres::graph) async fn plain_reads_ready(&self) -> Result<bool> {
        let pool = self.pool.get().await?;
        reads_ready(&pool).await
    }

    /// Writes follow the store reads use.
    ///
    /// Ledger `ready` → plain tables, even if Apache AGE is still installed.
    /// Dual-writing those edges into AGE drops them when the existing endpoint
    /// lives only in the plain tables (fresh v0.35.0, GitHub #477).
    /// Ledger `age` → AGE plus the plain mirror, which requires the probe in #477.
    pub(in crate::adapters::postgres::graph) async fn plain_writes(&self) -> Result<bool> {
        if self.plain_reads_ready().await? {
            return Ok(true);
        }
        Ok(!self.pool.graph_extension_available())
    }

    /// Backfill every AGE graph, shadow-compare 50 starts, then mark the ledger ready.
    pub async fn run_spec164_cutover(&self) -> Result<super::cutover::CutoverReport> {
        super::cutover::run_plain_graph_cutover(self).await
    }
}

/// `EDGEQUAKE_GRAPH_READ=age` keeps AGE reads before confirm-drop (EC-02).
pub(in crate::adapters::postgres::graph) fn graph_read_forced_age() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_GRAPH_READ")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "age"
    )
}

pub(in crate::adapters::postgres::graph) async fn tables_present(pool: &PgPool) -> Result<bool> {
    let present: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.eq_graph_nodes') IS NOT NULL
            AND to_regclass('public.eq_graph_cutover') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    Ok(present)
}

/// True only when the ledger row is `ready` and the operator has not pinned AGE.
pub(in crate::adapters::postgres::graph) async fn reads_ready(pool: &PgPool) -> Result<bool> {
    if graph_read_forced_age() {
        return Ok(false);
    }
    if !tables_present(pool).await? {
        return Ok(false);
    }
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_optional(pool)
            .await
            .map_err(|e| StorageError::Database(format!("eq_graph_cutover read failed: {e}")))?;
    Ok(status.as_deref() == Some("ready"))
}

/// Cypher MERGE does not fill plain tables (EC-03 / G6).
pub(in crate::adapters::postgres::graph) async fn refuse_cypher_opt_out(
    pool: &PgPool,
) -> Result<()> {
    if native_graph_writes_enabled() || !tables_present(pool).await? {
        return Ok(());
    }
    Err(StorageError::Database(
        "EDGEQUAKE_NATIVE_GRAPH_WRITES=0 is refused while SPEC-164 plain graph tables exist".into(),
    ))
}

/// Register this storage's namespace and attach AGE mirror triggers when the graph exists.
pub(in crate::adapters::postgres::graph) async fn register_graph(
    pool: &PgPool,
    graph_name: &str,
    namespace: &str,
) -> Result<()> {
    if !tables_present(pool).await? {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO public.eq_graph_namespace (graph_name, namespace)
         VALUES ($1, $2)
         ON CONFLICT (graph_name) DO UPDATE SET namespace = EXCLUDED.namespace",
    )
    .bind(graph_name)
    .bind(namespace)
    .execute(pool)
    .await
    .map_err(|e| StorageError::Database(format!("eq_graph_namespace upsert failed: {e}")))?;

    // Triggers are a no-op until Node/EDGE and eq_* columns exist.
    if let Err(e) = sqlx::query("SELECT public.eq_graph_install_mirror_triggers($1, $2)")
        .bind(graph_name)
        .bind(namespace)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, graph = graph_name, "SPEC-164 mirror trigger install skipped");
    }
    let _ = sqlx::query("SELECT public.eq_graph_install_plain_rls()")
        .execute(pool)
        .await;
    Ok(())
}
