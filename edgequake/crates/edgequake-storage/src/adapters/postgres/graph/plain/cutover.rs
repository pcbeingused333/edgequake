//! Backfill AGE label tables into plain rows, then shadow-compare neighbor sets.

use super::super::PostgresAGEGraphStorage;
use super::ledger::tables_present;
use crate::error::{Result, StorageError};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct CutoverReport {
    pub graphs: usize,
    pub mismatches: i64,
    pub ready: bool,
    pub age_absent: bool,
}

/// Finish migrate on the plain graph.
///
/// A second call is a no-op once `eq_graph_cutover.status` is `ready`, so
/// `edgequake migrate` stays idempotent and does not touch AGE label tables.
pub async fn finish_plain_graph_cutover(pool: &sqlx::PgPool) -> Result<CutoverReport> {
    if !super::ledger::tables_present(pool).await? {
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent: true,
        });
    }
    let row = sqlx::query("SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| StorageError::Database(format!("eq_graph_cutover read failed: {e}")))?;
    let (status, age_absent) = match row {
        Some(row) => (
            row.try_get::<String, _>("status").unwrap_or_default(),
            row.try_get::<bool, _>("age_absent").unwrap_or(false),
        ),
        None => (String::new(), false),
    };
    if status == "ready" {
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent,
        });
    }
    let config = crate::adapters::postgres::PostgresConfig::default().with_namespace("default");
    let wrapped =
        crate::adapters::postgres::PostgresPool::from_existing(pool.clone(), config.clone());
    wrapped.probe_graph_extension_available().await?;
    let storage = super::super::PostgresAGEGraphStorage::with_pool(wrapped, config);
    storage.run_spec164_cutover().await
}

pub(in crate::adapters::postgres::graph) async fn run_plain_graph_cutover(
    storage: &PostgresAGEGraphStorage,
) -> Result<CutoverReport> {
    let pool = storage.pool.get().await?;
    if !tables_present(&pool).await? {
        return Err(StorageError::Database(
            "SPEC-164 cutover requires migration 170 (eq_graph_nodes)".into(),
        ));
    }
    if !storage.pool.graph_extension_available() {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'ready', age_absent = true, shadow_ok = true,
                 backfill_complete = true, unbackfilled_graphs = 0, updated_at = now()
             WHERE id = 1",
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("greenfield ledger update failed: {e}")))?;
        return Ok(CutoverReport {
            graphs: 0,
            mismatches: 0,
            ready: true,
            age_absent: true,
        });
    }

    sqlx::query(
        "UPDATE public.eq_graph_cutover
         SET status = 'backfilling', updated_at = now()
         WHERE id = 1 AND status <> 'ready'",
    )
    .execute(&pool)
    .await
    .map_err(|e| StorageError::Database(format!("cutover status update failed: {e}")))?;

    let graphs: Vec<String> =
        sqlx::query_scalar("SELECT name::text FROM ag_catalog.ag_graph ORDER BY 1")
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("list ag_graph failed: {e}")))?;

    let mut mismatches = 0i64;
    for graph in &graphs {
        if !safe_ident(graph) {
            return Err(StorageError::Database(format!(
                "refusing unsafe graph name {graph}"
            )));
        }
        let namespace = namespace_for(&pool, graph).await?;
        let _ = sqlx::query("SELECT public.eq_graph_install_mirror_triggers($1, $2)")
            .bind(graph)
            .bind(&namespace)
            .execute(&pool)
            .await;
        let (nodes, edges, orphans) = copy_graph(&pool, graph, &namespace).await?;
        let graph_mismatches: i32 =
            sqlx::query_scalar("SELECT public.eq_graph_shadow_mismatches($1, $2)")
                .bind(&namespace)
                .bind(graph)
                .fetch_one(&pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("shadow compare failed for {graph}: {e}"))
                })?;
        mismatches += i64::from(graph_mismatches);
        let shadow_ok = graph_mismatches == 0;
        flush_plain_lineage_gin(&pool).await?;
        sqlx::query(
            "INSERT INTO public.eq_graph_backfill
                (graph_name, namespace, nodes_copied, edges_copied, orphans_skipped, shadow_ok, finished_at)
             VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $6 THEN now() ELSE NULL END)
             ON CONFLICT (graph_name) DO UPDATE SET
                namespace = EXCLUDED.namespace,
                nodes_copied = EXCLUDED.nodes_copied,
                edges_copied = EXCLUDED.edges_copied,
                orphans_skipped = EXCLUDED.orphans_skipped,
                shadow_ok = EXCLUDED.shadow_ok,
                finished_at = EXCLUDED.finished_at",
        )
        .bind(graph)
        .bind(&namespace)
        .bind(nodes)
        .bind(edges)
        .bind(orphans)
        .bind(shadow_ok)
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("backfill ledger write failed: {e}")))?;
    }

    let ready = mismatches == 0;
    if ready {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'ready', shadow_ok = true, backfill_complete = true,
                 unbackfilled_graphs = 0, age_absent = false, updated_at = now(),
                 detail = jsonb_build_object('graphs', $1::int, 'mismatches', 0)
             WHERE id = 1",
        )
        .bind(graphs.len() as i32)
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("cutover ready update failed: {e}")))?;
    } else {
        sqlx::query(
            "UPDATE public.eq_graph_cutover
             SET status = 'age', shadow_ok = false, backfill_complete = false, updated_at = now(),
                 detail = jsonb_build_object('mismatches', $1::bigint)
             WHERE id = 1",
        )
        .bind(mismatches)
        .execute(&pool)
        .await
        .ok();
    }

    Ok(CutoverReport {
        graphs: graphs.len(),
        mismatches,
        ready,
        age_absent: false,
    })
}

async fn namespace_for(pool: &sqlx::PgPool, graph: &str) -> Result<String> {
    let mapped: Option<String> =
        sqlx::query_scalar("SELECT namespace FROM public.eq_graph_namespace WHERE graph_name = $1")
            .bind(graph)
            .fetch_optional(pool)
            .await
            .map_err(|e| StorageError::Database(format!("namespace lookup failed: {e}")))?;
    if let Some(ns) = mapped {
        return Ok(ns);
    }
    Ok(derive_namespace(graph))
}

fn derive_namespace(graph: &str) -> String {
    let prefix = "eq_eq_";
    let suffix = "_graph";
    if let Some(rest) = graph.strip_prefix(prefix) {
        if let Some(ns) = rest.strip_suffix(suffix) {
            return ns.to_string();
        }
    }
    graph.to_string()
}

async fn copy_graph(pool: &sqlx::PgPool, graph: &str, namespace: &str) -> Result<(i64, i64, i64)> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| StorageError::Database(format!("backfill begin failed: {e}")))?;
    // Statement triggers would aggregate this whole copy. One recompute after
    // the load writes each degree once.
    sqlx::query("SELECT set_config('eq.skip_graph_degree', 'on', true)")
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("skip degree maintenance failed: {e}")))?;
    let node_sql = format!(
        r#"INSERT INTO public.eq_graph_nodes
             (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
           SELECT $1, d.nid,
                  COALESCE(d.props->>'tenant_id', ''),
                  COALESCE(d.props->>'workspace_id', ''),
                  NULLIF(d.props->>'entity_type', ''),
                  d.props
           FROM (
             SELECT DISTINCT ON (nid)
                    COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') AS nid,
                    ag_catalog.agtype_to_json(n.properties)::jsonb AS props
             FROM {graph}."Node" n
             WHERE COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') IS NOT NULL
             ORDER BY 1, n.ctid DESC
           ) d
           ON CONFLICT (namespace, node_id) DO UPDATE SET
             tenant_id = EXCLUDED.tenant_id,
             workspace_id = EXCLUDED.workspace_id,
             entity_type = EXCLUDED.entity_type,
             properties = public.eq_merge_graph_properties(
                 public.eq_graph_nodes.properties, EXCLUDED.properties)"#
    );
    let nodes = sqlx::query(&node_sql)
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("backfill nodes {graph} failed: {e}")))?
        .rows_affected() as i64;

    let orphan_sql = format!(
        r#"SELECT COUNT(*)::bigint FROM {graph}."EDGE" e
           WHERE NOT EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n
             WHERE n.namespace = $1
               AND n.node_id = COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id')
           ) OR NOT EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n
             WHERE n.namespace = $1
               AND n.node_id = COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id')
           )"#
    );
    let orphans: i64 = sqlx::query_scalar(&orphan_sql)
        .bind(namespace)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

    let edge_sql = format!(
        r#"INSERT INTO public.eq_graph_edges
             (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
           SELECT $1, d.src, d.tgt, d.rel,
                  COALESCE(d.props->>'tenant_id', ''),
                  COALESCE(d.props->>'workspace_id', ''),
                  CASE WHEN COALESCE(d.props->>'weight', '') ~ '^-?[0-9]+(\.[0-9]+)?$'
                       THEN (d.props->>'weight')::float8 ELSE NULL END,
                  d.props
           FROM (
             SELECT DISTINCT ON (src, tgt, rel)
                    COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id') AS src,
                    COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') AS tgt,
                    COALESCE(NULLIF(e.eq_rel_type, ''), NULLIF(ag_catalog.agtype_to_json(e.properties)->>'relation_type', ''), '') AS rel,
                    ag_catalog.agtype_to_json(e.properties)::jsonb AS props
             FROM {graph}."EDGE" e
           ) d
           WHERE EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.src
           ) AND EXISTS (
             SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.tgt
           )
           ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
             properties = public.eq_merge_graph_properties(
                 public.eq_graph_edges.properties, EXCLUDED.properties),
             tenant_id = EXCLUDED.tenant_id,
             workspace_id = EXCLUDED.workspace_id,
             weight = EXCLUDED.weight"#
    );
    let edges = sqlx::query(&edge_sql)
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("backfill edges {graph} failed: {e}")))?
        .rows_affected() as i64;
    sqlx::query("SELECT public.eq_graph_recompute_degrees($1)")
        .bind(namespace)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("recompute degrees {graph} failed: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| StorageError::Database(format!("backfill commit failed: {e}")))?;
    Ok((nodes, edges, orphans))
}

async fn flush_plain_lineage_gin(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query(
        "SELECT
           gin_clean_pending_list('public.idx_eq_graph_nodes_source_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_source_chunk_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_edges_source_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_edges_source_chunk_ids_gin'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_id_trgm'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_desc_trgm'::regclass),
           gin_clean_pending_list('public.idx_eq_graph_nodes_type_trgm'::regclass)",
    )
    .execute(pool)
    .await
    .map_err(|e| StorageError::Database(format!("lineage GIN flush failed: {e}")))?;
    Ok(())
}

fn safe_ident(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
