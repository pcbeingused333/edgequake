//! Counts and workspace stats (Q-GRAPH-019, 024..026).

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod counts {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn node_count(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<usize> {
        count_ns(storage, "public.eq_graph_nodes", None).await
    }

    pub(in crate::adapters::postgres::graph) async fn edge_count(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<usize> {
        count_ns(storage, "public.eq_graph_edges", None).await
    }

    pub(in crate::adapters::postgres::graph) async fn node_count_by_workspace(
        storage: &PostgresAGEGraphStorage,
        workspace_id: &str,
    ) -> Result<usize> {
        count_ns(storage, "public.eq_graph_nodes", Some(workspace_id)).await
    }

    pub(in crate::adapters::postgres::graph) async fn edge_count_by_workspace(
        storage: &PostgresAGEGraphStorage,
        workspace_id: &str,
    ) -> Result<usize> {
        count_ns(storage, "public.eq_graph_edges", Some(workspace_id)).await
    }

    pub(in crate::adapters::postgres::graph) async fn distinct_types(
        storage: &PostgresAGEGraphStorage,
        workspace_id: &str,
    ) -> Result<usize> {
        let pool = storage.pool.get().await?;
        // GROUP BY walks idx_eq_graph_nodes_ws_type. COUNT(DISTINCT) sorted the heap.
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM (
                SELECT entity_type
                FROM public.eq_graph_nodes
                WHERE namespace = $1 AND workspace_id = $2
                  AND entity_type IS NOT NULL AND entity_type <> ''
                GROUP BY entity_type
             ) types",
        )
        .bind(&storage.namespace)
        .bind(workspace_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain distinct types failed: {e}")))?;
        Ok(n.max(0) as usize)
    }
}

async fn count_ns(
    storage: &PostgresAGEGraphStorage,
    table: &str,
    workspace_id: Option<&str>,
) -> Result<usize> {
    let pool = storage.pool.get().await?;
    let (sql, bind_ws) = if workspace_id.is_some() {
        (
            format!(
                "SELECT COUNT(*)::bigint FROM {table} WHERE namespace = $1 AND workspace_id = $2"
            ),
            true,
        )
    } else {
        (
            format!("SELECT COUNT(*)::bigint FROM {table} WHERE namespace = $1"),
            false,
        )
    };
    let mut q = sqlx::query(&sql).bind(&storage.namespace);
    if bind_ws {
        q = q.bind(workspace_id.unwrap_or(""));
    }
    let row = q
        .fetch_one(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain count failed: {e}")))?;
    let n: i64 = row.get(0);
    Ok(n.max(0) as usize)
}
