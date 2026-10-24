//! Search, labels, and popular nodes (Q-GRAPH-008..010).

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use super::rows::{lit, node_from_row, tenant_and, TenantMode};
use crate::error::{Result, StorageError};
use crate::traits::GraphNode;

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod search {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn search_nodes(
        storage: &PostgresAGEGraphStorage,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let pool = storage.pool.get().await?;
        if let Some(session) = crate::clearance_session::ClearanceSession::current() {
            if !session.predicate_sql().is_empty() {
                return search_visible_contributions(&pool, query, limit, workspace_id, &session)
                    .await;
            }
        }
        // Migration 206. LIKE is not leakproof, so the tenant role cannot use
        // the trigram indexes. The owner function applies the session tenant
        // and keeps ORDER BY degree DESC, node_id.
        let rows = sqlx::query(SEARCH_NODES_SQL)
            .bind(&storage.namespace)
            .bind(query.to_lowercase())
            .bind(i32::try_from(limit.max(1)).unwrap_or(i32::MAX))
            .bind(entity_type)
            .bind(tenant_id)
            .bind(workspace_id)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain search_nodes failed: {e}")))?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let node = node_from_row(row)?;
                let degree: i64 = row.try_get("degree").unwrap_or(0);
                Some((node, degree.max(0) as usize))
            })
            .collect())
    }

    pub(in crate::adapters::postgres::graph) async fn popular_nodes(
        storage: &PostgresAGEGraphStorage,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        if let Some(session) = crate::clearance_session::ClearanceSession::current() {
            if !session.predicate_sql().is_empty() {
                let pool = storage.pool.get().await?;
                return search_visible_contributions(&pool, "", limit, workspace_id, &session)
                    .await;
            }
        }
        let scope = tenant_and("n", tenant_id, workspace_id, TenantMode::Strict);
        let etype = entity_type
            .map(|t| {
                format!(
                    " AND UPPER(COALESCE(n.entity_type, '')) = UPPER('{}')",
                    lit(t)
                )
            })
            .unwrap_or_default();
        let min_sql = min_degree
            .map(|d| format!(" AND n.degree >= {d}"))
            .unwrap_or_default();
        let pool = storage.pool.get().await?;
        // Index (namespace, degree DESC, node_id) returns LIMIT rows and stops.
        // Hash-aggregating both ends of every edge was 616ms at 200k/500k.
        let sql = format!(
            "SELECT n.node_id, n.properties, n.degree
             FROM public.eq_graph_nodes n
             WHERE n.namespace = $1 {scope} {etype} {min_sql}
             ORDER BY n.degree DESC, n.node_id
             LIMIT {limit}"
        );
        let rows = sqlx::query(&sql)
            .bind(&storage.namespace)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain popular_nodes failed: {e}")))?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let node = node_from_row(row)?;
                let degree: i64 = row.try_get("degree").unwrap_or(0);
                Some((node, degree.max(0) as usize))
            })
            .collect())
    }

    async fn search_visible_contributions(
        pool: &sqlx::PgPool,
        query: &str,
        limit: usize,
        workspace_id: Option<&str>,
        session: &crate::clearance_session::ClearanceSession,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let Some(workspace) = workspace_id.and_then(|id| uuid::Uuid::parse_str(id).ok()) else {
            return Ok(Vec::new());
        };
        let mut conn = pool.acquire().await.map_err(|error| {
            StorageError::Database(format!("clearance graph search failed: {error}"))
        })?;
        crate::adapters::postgres::rls::apply_clearance_session(&mut conn, session)
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);
        let rows = sqlx::query(
            "SELECT entity_name, (array_agg(fragment ORDER BY contribution_id))[1] AS fragment, count(*) AS degree
             FROM public.graph_contributions
             WHERE workspace_id = $1
               AND kind IN ('entity', 'node')
               AND entity_name ILIKE '%' || $2 || '%'
               AND (public.clearance_covers(classification_rank, compartment_ids)
                    OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))
             GROUP BY entity_name
             ORDER BY count(*) DESC, entity_name
             LIMIT $3",
        )
        .bind(workspace)
        .bind(query.to_lowercase())
        .bind(limit)
        .fetch_all(&mut *conn)
        .await
        .map_err(|error| StorageError::Database(format!("clearance graph search failed: {error}")))?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let name: String = row.try_get("entity_name").ok()?;
                let fragment: Option<String> = row.try_get("fragment").ok();
                let degree: i64 = row.try_get("degree").unwrap_or(0);
                let mut node = GraphNode::new(name);
                if let Some(fragment) = fragment {
                    node.properties.insert(
                        "description".to_string(),
                        serde_json::Value::String(fragment),
                    );
                }
                Some((node, degree.max(0) as usize))
            })
            .collect())
    }

    /// Migration 206. `LIKE` stays inside `eq_graph_search_nodes`.
    const SEARCH_NODES_SQL: &str = "SELECT node_id, properties, degree \
         FROM public.eq_graph_search_nodes($1, $2, $3, $4, $5, $6)";

    /// Migration 204. `LIKE` stays inside `eq_graph_search_labels`, which
    /// runs as the table owner so the trigram index is usable under forced
    /// row security.
    const SEARCH_LABELS_SQL: &str =
        "SELECT label FROM public.eq_graph_search_labels($1, $2, $3, $4, $5)";

    /// Migration 210. Typo recovery when the substring function returns
    /// nothing. The `%` trigram operator is not leakproof, so it stays inside
    /// an owner function: as the tenant role it filtered every node of the
    /// workspace (835 ms at 66k nodes) instead of using
    /// `idx_eq_graph_nodes_id_trgm` (28 ms).
    const SEARCH_LABELS_TYPO_SQL: &str =
        "SELECT label FROM public.eq_graph_search_labels_fuzzy($1, $2, $3, $4, $5)";

    pub(in crate::adapters::postgres::graph) async fn search_labels(
        storage: &PostgresAGEGraphStorage,
        query: &str,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let pool = storage.pool.get().await?;
        // Migration 204. `LIKE` is not leakproof, so the tenant role cannot
        // use the trigram indexes. The function runs as the table owner and
        // keeps the session tenant.
        let limit = i32::try_from(limit).unwrap_or(i32::MAX);
        let query = query.to_lowercase();
        let rows = sqlx::query_scalar::<_, String>(SEARCH_LABELS_SQL)
            .bind(&storage.namespace)
            .bind(&query)
            .bind(limit)
            .bind(tenant_id)
            .bind(workspace_id)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain search_labels failed: {e}")))?;
        if !rows.is_empty() {
            return Ok(rows);
        }
        // Migration 204's owner function matches substrings. An interior typo
        // is a trigram match on the same index, still inside the tenant scope.
        let fuzzy = sqlx::query_scalar::<_, String>(SEARCH_LABELS_TYPO_SQL)
            .bind(&storage.namespace)
            .bind(&query)
            .bind(limit)
            .bind(tenant_id)
            .bind(workspace_id)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain search_labels typo failed: {e}")))?;
        Ok(fuzzy)
    }

    pub(in crate::adapters::postgres::graph) async fn popular_labels(
        storage: &PostgresAGEGraphStorage,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let scope = tenant_and("n", tenant_id, workspace_id, TenantMode::Strict);
        let pool = storage.pool.get().await?;
        let sql = format!(
            "SELECT n.entity_type
             FROM public.eq_graph_nodes n
             WHERE n.namespace = $1 {scope} AND n.entity_type IS NOT NULL AND n.entity_type <> ''
             GROUP BY n.entity_type
             ORDER BY COUNT(*) DESC, n.entity_type
             LIMIT {limit}"
        );
        let rows = sqlx::query_scalar::<_, String>(&sql)
            .bind(&storage.namespace)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain popular_labels failed: {e}")))?;
        Ok(rows)
    }

    #[cfg(test)]
    mod tests {
        use super::{SEARCH_LABELS_SQL, SEARCH_LABELS_TYPO_SQL, SEARCH_NODES_SQL};

        #[test]
        fn search_labels_goes_through_the_owner_function() {
            assert!(SEARCH_LABELS_SQL.contains("eq_graph_search_labels"));
            assert!(!SEARCH_LABELS_SQL.contains("LIKE"));
        }

        #[test]
        fn typo_labels_go_through_the_owner_function_with_the_caller_scope() {
            // `%` is not leakproof, so under forced RLS it only reaches the
            // trigram index inside the SECURITY DEFINER function (migration
            // 210), which also lets the session scope win over these binds.
            assert!(SEARCH_LABELS_TYPO_SQL.contains("eq_graph_search_labels_fuzzy"));
            for bind in ["$1", "$2", "$3", "$4", "$5"] {
                assert!(SEARCH_LABELS_TYPO_SQL.contains(bind), "missing {bind}");
            }
            assert!(!SEARCH_LABELS_TYPO_SQL.contains(" % "));
            assert!(!SEARCH_LABELS_TYPO_SQL.contains("ag_catalog"));
        }

        #[test]
        fn search_nodes_goes_through_the_owner_function() {
            assert!(SEARCH_NODES_SQL.contains("eq_graph_search_nodes"));
            assert!(!SEARCH_NODES_SQL.contains("LIKE"));
        }
    }
}
