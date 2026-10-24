//! Point get, incident edges, degree, and BFS close (Q-GRAPH-001..007).

use std::collections::HashMap;

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use super::rows::{edge_from_row, node_from_row, tenant_and, TenantMode};
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode};

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod read {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn get_nodes_batch(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<HashMap<String, GraphNode>> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(super::super::super::helpers::PLAIN_GET_NODES_BATCH_SQL)
            .bind(&storage.namespace)
            .bind(node_ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain get_nodes_batch failed: {e}")))?;
        let mut nodes: HashMap<String, GraphNode> = rows
            .iter()
            .filter_map(node_from_row)
            .map(|n| (n.id.clone(), n))
            .collect();
        project_visible_descriptions(storage, &pool, &mut nodes).await?;
        Ok(nodes)
    }

    pub(in crate::adapters::postgres::graph) async fn node_degrees_batch(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<Vec<(String, usize)>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let pool = storage.pool.get().await?;
        if let Some(session) = crate::clearance_session::ClearanceSession::current() {
            if !session.predicate_sql().is_empty() {
                return visible_degrees(storage, &pool, node_ids, &session).await;
            }
        }
        let rows = sqlx::query(
            "SELECT id AS node_id, COALESCE(n.degree, 0)::bigint AS degree
             FROM unnest($2::text[]) AS id
             LEFT JOIN public.eq_graph_nodes n
               ON n.namespace = $1 AND n.node_id = id",
        )
        .bind(&storage.namespace)
        .bind(node_ids)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain node_degrees_batch failed: {e}")))?;
        let mut map: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let id: String = row.get("node_id");
            let degree: i64 = row.get("degree");
            map.insert(id, degree.max(0) as usize);
        }
        Ok(node_ids
            .iter()
            .map(|id| (id.clone(), map.get(id).copied().unwrap_or(0)))
            .collect())
    }

    async fn project_visible_descriptions(
        storage: &PostgresAGEGraphStorage,
        pool: &sqlx::PgPool,
        nodes: &mut HashMap<String, GraphNode>,
    ) -> Result<()> {
        let Some(session) = crate::clearance_session::ClearanceSession::current() else {
            return Ok(());
        };
        if session.predicate_sql().is_empty() || nodes.is_empty() {
            return Ok(());
        }
        let Some(workspace) = uuid::Uuid::parse_str(&storage.namespace).ok() else {
            for node in nodes.values_mut() {
                node.properties.remove("description");
            }
            return Ok(());
        };
        let ids: Vec<String> = nodes.keys().cloned().collect();
        let mut conn = pool.acquire().await.map_err(|error| {
            StorageError::Database(format!("clearance description read failed: {error}"))
        })?;
        crate::adapters::postgres::rls::apply_clearance_session(&mut conn, &session)
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        let rows = sqlx::query(
            "SELECT entity_name, (array_agg(fragment ORDER BY contribution_id))[1] AS fragment
             FROM public.graph_contributions
             WHERE workspace_id = $1
               AND entity_name = ANY($2)
               AND kind IN ('entity', 'node')
               AND (public.clearance_covers(classification_rank, compartment_ids)
                    OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))
             GROUP BY entity_name",
        )
        .bind(workspace)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(|error| {
            StorageError::Database(format!("clearance description read failed: {error}"))
        })?;
        let mut fragments: HashMap<String, String> = HashMap::new();
        for row in &rows {
            let name: String = row.get("entity_name");
            let fragment: Option<String> = row.get("fragment");
            if let Some(fragment) = fragment {
                fragments.insert(name, fragment);
            }
        }
        for node in nodes.values_mut() {
            match fragments.get(&node.id) {
                Some(fragment) => {
                    node.properties.insert(
                        "description".to_string(),
                        serde_json::Value::String(fragment.clone()),
                    );
                }
                None => {
                    node.properties.remove("description");
                }
            }
        }
        Ok(())
    }

    async fn visible_degrees(
        storage: &PostgresAGEGraphStorage,
        pool: &sqlx::PgPool,
        node_ids: &[String],
        session: &crate::clearance_session::ClearanceSession,
    ) -> Result<Vec<(String, usize)>> {
        let Some(workspace) = uuid::Uuid::parse_str(&storage.namespace).ok() else {
            return Ok(node_ids.iter().cloned().map(|id| (id, 0)).collect());
        };
        let mut conn = pool.acquire().await.map_err(|error| {
            StorageError::Database(format!("clearance degree read failed: {error}"))
        })?;
        crate::adapters::postgres::rls::apply_clearance_session(&mut conn, session)
            .await
            .map_err(|error| StorageError::Database(error.to_string()))?;
        let rows = sqlx::query(
            "SELECT entity_name, count(*)::bigint AS degree
             FROM public.graph_contributions
             WHERE workspace_id = $1
               AND entity_name = ANY($2)
               AND (public.clearance_covers(classification_rank, compartment_ids)
                    OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))
             GROUP BY entity_name",
        )
        .bind(workspace)
        .bind(node_ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(|error| StorageError::Database(format!("clearance degree read failed: {error}")))?;
        let mut map: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let name: String = row.get("entity_name");
            let degree: i64 = row.get("degree");
            map.insert(name, degree.max(0) as usize);
        }
        Ok(node_ids
            .iter()
            .map(|id| (id.clone(), map.get(id).copied().unwrap_or(0)))
            .collect())
    }

    pub(in crate::adapters::postgres::graph) async fn get_all_nodes(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<Vec<GraphNode>> {
        let pool = storage.pool.get().await?;
        if let Some(session) = crate::clearance_session::ClearanceSession::current() {
            if !session.predicate_sql().is_empty() {
                let found = crate::adapters::postgres::graph::plain::search::search::search_nodes(
                    storage,
                    "",
                    10_000,
                    None,
                    None,
                    Some(storage.namespace.as_str()),
                )
                .await?;
                return Ok(found.into_iter().map(|(node, _)| node).collect());
            }
        }
        let rows = sqlx::query(
            "SELECT node_id, properties FROM public.eq_graph_nodes WHERE namespace = $1",
        )
        .bind(&storage.namespace)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_all_nodes failed: {e}")))?;
        Ok(rows.iter().filter_map(node_from_row).collect())
    }

    pub(in crate::adapters::postgres::graph) async fn incident_edges(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let scope = tenant_and("e", tenant_id, workspace_id, TenantMode::Strict);
        let pool = storage.pool.get().await?;
        let mut unique = node_ids.to_vec();
        unique.sort();
        unique.dedup();
        const CHUNK: usize = 200;
        let mut edges = Vec::new();
        for chunk in unique.chunks(CHUNK) {
            let sql = format!(
                "SELECT source_id, target_id, properties
                 FROM public.eq_graph_edges e
                 WHERE e.namespace = $1
                   AND (e.source_id = ANY($2::text[]) OR e.target_id = ANY($2::text[]))
                   {scope}"
            );
            let rows = sqlx::query(&sql)
                .bind(&storage.namespace)
                .bind(chunk)
                .fetch_all(&pool)
                .await
                .map_err(|e| StorageError::Database(format!("plain incident edges failed: {e}")))?;
            edges.extend(rows.iter().filter_map(edge_from_row));
        }
        if let Some(session) = crate::clearance_session::ClearanceSession::current() {
            if !session.predicate_sql().is_empty() {
                let mut ids: Vec<String> = edges
                    .iter()
                    .flat_map(|edge| [edge.source.clone(), edge.target.clone()])
                    .collect();
                ids.sort();
                ids.dedup();
                let visible: HashMap<String, usize> =
                    visible_degrees(storage, &pool, &ids, &session)
                        .await?
                        .into_iter()
                        .collect();
                edges.retain(|edge| {
                    visible.get(&edge.source).copied().unwrap_or(0) > 0
                        && visible.get(&edge.target).copied().unwrap_or(0) > 0
                });
            }
        }
        Ok(edges)
    }

    pub(in crate::adapters::postgres::graph) async fn get_edge(
        storage: &PostgresAGEGraphStorage,
        source: &str,
        target: &str,
    ) -> Result<Option<GraphEdge>> {
        let pool = storage.pool.get().await?;
        let row = sqlx::query(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges
             WHERE namespace = $1 AND source_id = $2 AND target_id = $3
             LIMIT 1",
        )
        .bind(&storage.namespace)
        .bind(source)
        .bind(target)
        .fetch_optional(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_edge failed: {e}")))?;
        Ok(row.as_ref().and_then(edge_from_row))
    }

    pub(in crate::adapters::postgres::graph) async fn get_all_edges(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<Vec<GraphEdge>> {
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges WHERE namespace = $1",
        )
        .bind(&storage.namespace)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_all_edges failed: {e}")))?;
        Ok(rows.iter().filter_map(edge_from_row).collect())
    }

    /// Both endpoints in the set (Q-GRAPH-006 close). Legacy empty tenant matches.
    pub(in crate::adapters::postgres::graph) async fn edges_for_node_set(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let scope = tenant_and("e", tenant_id, workspace_id, TenantMode::LegacyEmpty);
        let pool = storage.pool.get().await?;
        let sql = format!(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges e
             WHERE e.namespace = $1
               AND e.source_id = ANY($2::text[])
               AND e.target_id = ANY($2::text[])
               {scope}"
        );
        let rows = sqlx::query(&sql)
            .bind(&storage.namespace)
            .bind(node_ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain edges_for_node_set failed: {e}")))?;
        Ok(rows.iter().filter_map(edge_from_row).collect())
    }

    pub(in crate::adapters::postgres::graph) async fn nodes_with_degrees(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        let nodes = get_nodes_batch(storage, node_ids).await?;
        let pool = storage.pool.get().await?;
        // Two index-only counts. An OR join fetches every incident edge row
        // just to count it. A self-loop is stored once and counted on both sides.
        let rows = sqlx::query(
            "SELECT id AS node_id,
                    (SELECT COUNT(*) FROM public.eq_graph_edges e
                      WHERE e.namespace = $1 AND e.target_id = id)::bigint AS in_degree,
                    (SELECT COUNT(*) FROM public.eq_graph_edges e
                      WHERE e.namespace = $1 AND e.source_id = id)::bigint AS out_degree
             FROM unnest($2::text[]) AS id",
        )
        .bind(&storage.namespace)
        .bind(node_ids)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain nodes_with_degrees failed: {e}")))?;
        let mut deg: HashMap<String, (usize, usize)> = HashMap::new();
        for row in &rows {
            let id: String = row.get("node_id");
            let out_d: i64 = row.get("out_degree");
            let in_d: i64 = row.get("in_degree");
            deg.insert(id, (out_d.max(0) as usize, in_d.max(0) as usize));
        }
        Ok(node_ids
            .iter()
            .filter_map(|id| {
                let node = nodes.get(id)?.clone();
                let (out_d, in_d) = deg.get(id).copied().unwrap_or((0, 0));
                Some((node, in_d, out_d))
            })
            .collect())
    }
}
