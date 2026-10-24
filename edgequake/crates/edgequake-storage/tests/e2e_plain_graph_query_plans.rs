//! Planner proof for the serving-graph reads and the node upsert.
//!
//! Uses the scratch test database. Skips when `DATABASE_URL` is unset.
//! `EXPLAIN` does not write. The fixture rows are deleted before return.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::pdf_storage::PDF_CONTENT_VIEW_SQL;
use edgequake_storage::{
    plain_node_counts_by_source_prefixes_sql, PLAIN_GET_NODES_BATCH_SQL,
    PLAIN_NAMESPACE_NODE_COUNT_SQL, PLAIN_NODE_IDS_PRESENT_SQL, PLAIN_NODE_UPSERT_SQL,
};
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

fn plan_text(rows: &[sqlx::postgres::PgRow]) -> String {
    rows.iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>()
        .join("\n")
}

fn namespace_led(plan: &str) -> bool {
    (plan.contains("Index Scan") || plan.contains("Index Only Scan"))
        && plan.contains("namespace =")
        && (plan.contains("eq_graph_nodes_pkey")
            || plan.contains("idx_eq_graph_nodes_degree")
            || plan.contains("idx_eq_graph_nodes_scope_node"))
}

#[tokio::test]
async fn serving_graph_queries_use_their_indexes() {
    let Some(cfg) = require_or_skip_postgres("planprobe") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let bulk = format!("planbulk_{}", Uuid::new_v4().simple());
    let rare = format!("planrare_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, properties)
         SELECT $1, 'n' || i::text,
                jsonb_build_object(
                    'source_ids', jsonb_build_array('doc-chunk-' || i::text),
                    'source_chunk_ids', jsonb_build_array('doc-chunk-' || i::text)
                )
         FROM generate_series(0, 3999) AS i",
    )
    .bind(&bulk)
    .execute(&pool)
    .await
    .expect("seed bulk");
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, properties)
         VALUES ($1, 'only', '{}'::jsonb)",
    )
    .bind(&rare)
    .execute(&pool)
    .await
    .expect("seed rare");
    sqlx::query("ANALYZE public.eq_graph_nodes")
        .execute(&pool)
        .await
        .expect("analyze");

    let result = run_explains(&pool, &bulk, &rare).await;

    sqlx::query("DELETE FROM public.eq_graph_nodes WHERE namespace = $1 OR namespace = $2")
        .bind(&bulk)
        .bind(&rare)
        .execute(&pool)
        .await
        .expect("cleanup");

    result.expect("plans");
}

async fn run_explains(pool: &PgPool, bulk: &str, rare: &str) -> Result<(), String> {
    let prefix_sql = format!(
        "EXPLAIN (FORMAT TEXT) {}",
        plain_node_counts_by_source_prefixes_sql()
    );
    let prefix_rows = sqlx::query(&prefix_sql)
        .bind(vec!["doc-chunk-".to_string()])
        .bind(1_i32)
        .bind(bulk)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let prefix = plan_text(&prefix_rows);
    if !prefix.contains("idx_eq_graph_nodes_source_ids_gin")
        || !prefix.contains("idx_eq_graph_nodes_source_chunk_ids_gin")
        || prefix.contains("Seq Scan on eq_graph_nodes")
        || prefix.contains("Seq Scan on public.eq_graph_nodes")
    {
        return Err(prefix);
    }

    let bulk_sql = format!("EXPLAIN (FORMAT TEXT) {PLAIN_NAMESPACE_NODE_COUNT_SQL}");
    let bulk_rows = sqlx::query(&bulk_sql)
        .bind(bulk)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let bulk_plan = plan_text(&bulk_rows);
    if !bulk_plan.contains("eq_graph_nodes")
        || (!namespace_led(&bulk_plan) && !bulk_plan.contains("Seq Scan"))
    {
        return Err(bulk_plan);
    }
    if bulk_plan.contains("idx_eq_graph_nodes_source") {
        return Err(bulk_plan);
    }

    let rare_sql = format!("EXPLAIN (FORMAT TEXT) {PLAIN_NAMESPACE_NODE_COUNT_SQL}");
    let rare_rows = sqlx::query(&rare_sql)
        .bind(rare)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let rare_plan = plan_text(&rare_rows);
    if !namespace_led(&rare_plan) || rare_plan.contains("Seq Scan") {
        return Err(rare_plan);
    }

    let present_sql = format!("EXPLAIN (FORMAT TEXT) {PLAIN_NODE_IDS_PRESENT_SQL}");
    let present_rows = sqlx::query(&present_sql)
        .bind(bulk)
        .bind(vec!["n0".to_string()])
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let present = plan_text(&present_rows);
    if !present.contains("eq_graph_nodes_pkey") || present.contains("Seq Scan") {
        return Err(present);
    }

    let batch_sql = format!("EXPLAIN (FORMAT TEXT) {PLAIN_GET_NODES_BATCH_SQL}");
    let batch_rows = sqlx::query(&batch_sql)
        .bind(bulk)
        .bind(vec!["n0".to_string(), "n1".to_string()])
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let batch = plan_text(&batch_rows);
    if !batch.contains("eq_graph_nodes_pkey") || batch.contains("Seq Scan") {
        return Err(batch);
    }

    let upsert_sql = format!("EXPLAIN (FORMAT TEXT) {PLAIN_NODE_UPSERT_SQL}");
    let upsert_rows = sqlx::query(&upsert_sql)
        .bind(bulk)
        .bind(vec!["n0".to_string()])
        .bind(vec!["t".to_string()])
        .bind(vec!["w".to_string()])
        .bind(vec!["PERSON".to_string()])
        .bind(vec![r#"{"source_ids":["doc-chunk-0"]}"#.to_string()])
        .bind(false)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let upsert = plan_text(&upsert_rows);
    if !upsert.contains("eq_graph_nodes_pkey") {
        return Err(upsert);
    }

    if PDF_CONTENT_VIEW_SQL.contains("pdf_data") {
        return Err(PDF_CONTENT_VIEW_SQL.to_string());
    }
    let content_sql = format!("EXPLAIN (FORMAT TEXT) {PDF_CONTENT_VIEW_SQL}");
    let content_rows = sqlx::query(&content_sql)
        .bind(Uuid::nil())
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let content = plan_text(&content_rows);
    if !content.contains("pdf_documents_pkey") {
        return Err(content);
    }
    Ok(())
}
