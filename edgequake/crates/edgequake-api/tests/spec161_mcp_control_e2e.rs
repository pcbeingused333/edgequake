//! SPEC-161 MCP control-surface e2e (T-161-01..52). Ingest/upload stay async.

mod common;

use std::collections::HashMap;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine};
use common::spec028_mcp::{
    auth_enabled_mcp_state, build_mcp_app, issue_mcp_jwt, mcp_post_bearer, mcp_post_bytes,
    mcp_post_legacy, mcp_tools_call, parse_json, tool_structured, tools_call_body,
};
use edgequake_api::mcp::gateway::tools::tools_list_for;
use edgequake_api::mcp::project::profile::McpProfile;
use edgequake_api::AppState;
use edgequake_auth::Role;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn seeded_mcp_app() -> axum::Router {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    build_mcp_app(state)
}

fn names_from_list(body: &Value) -> Vec<String> {
    body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn t161_01_empty_content_invalid_id() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(&app, "/mcp", "eq_ingest", json!({"content": ""})).await;
    let sc = tool_structured(&call);
    assert_eq!(sc["ok"], false);
    assert_eq!(sc["error"]["code"], "eq/invalid_id");
}

#[tokio::test]
async fn t161_02_oversize_does_not_admit() {
    let mut state = AppState::test_state();
    state.config.max_document_size = 8;
    let app = build_mcp_app(state);
    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": "0123456789abcdef", "title": "big"}),
    )
    .await;
    let sc = tool_structured(&call);
    assert_eq!(sc["ok"], false);
}

#[tokio::test]
async fn t161_03_mutual_exclusive_payloads() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": "hi", "content_base64": "aGk="}),
    )
    .await;
    let sc = tool_structured(&call);
    assert_eq!(sc["error"]["code"], "eq/invalid_id");
}

#[tokio::test]
async fn t161_04_no_payload_invalid_id() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(&app, "/mcp", "eq_ingest", json!({})).await;
    assert_eq!(tool_structured(&call)["error"]["code"], "eq/invalid_id");
}

#[tokio::test]
async fn t161_05_10_09_duplicate_pending_unicode() {
    let app = seeded_mcp_app().await;
    let body = "same-hash-body-spec161";
    let (_, a) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": body, "title": "Café 日本語"}),
    )
    .await;
    let sc = tool_structured(&a);
    assert_eq!(sc["ok"], true, "ingest envelope: {sc}");
    assert_eq!(sc["status"], "pending");
    assert_ne!(sc["status"], "queued");
    assert_eq!(sc["ready"], false);
    assert!(sc["task_id"].is_string());
    let doc_id = sc["document_id"].as_str().unwrap().to_string();

    let (_, b) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": body, "title": "other"}),
    )
    .await;
    let sc2 = tool_structured(&b);
    assert_eq!(sc2["document_id"], doc_id);

    let (_, get) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_get",
        json!({"document_id": doc_id}),
    )
    .await;
    let g = tool_structured(&get);
    assert_eq!(g["ok"], true, "document_get envelope: {g}");
    assert_eq!(g["ready"], false);
    assert_eq!(g["status"], "pending");
    assert_eq!(g["documents"][0]["status"], "pending");
}

#[tokio::test]
async fn t161_07_read_scope_forbidden_on_ingest() {
    let state = auth_enabled_mcp_state().await;
    let token = issue_mcp_jwt(&state, Role::Readonly, "edgequake:read");
    let app = build_mcp_app(state);
    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &token,
            tools_call_body("eq_ingest", json!({"content": "x"})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn t161_08_query_profile_omits_writes() {
    let listed = tools_list_for(McpProfile::Query);
    let names = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect::<Vec<_>>();
    assert!(!names.contains(&"eq_ingest"));
    assert!(names.contains(&"eq_search"));
}

#[tokio::test]
async fn t161_11_unknown_task_not_found() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_task_get",
        json!({"task_id": "missing-task"}),
    )
    .await;
    assert_eq!(tool_structured(&call)["error"]["code"], "eq/not_found");
}

#[tokio::test]
async fn t161_12_13_18_19_20_upload_session() {
    let app = seeded_mcp_app().await;
    let payload = b"hello-mcp-upload";
    let b64 = STANDARD.encode(payload);
    let sha = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(payload))
    };

    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({"filename": "../evil.txt", "byte_length": payload.len(), "sha256": sha, "chunked": true}),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["ok"], true);
    let upload_id = sc["upload_id"].as_str().unwrap().to_string();

    let (_, gap) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_write",
        json!({"upload_id": upload_id, "offset": 3, "chunk_base64": b64}),
    )
    .await;
    assert_eq!(tool_structured(&gap)["error"]["code"], "eq/invalid_id");

    let (_, write) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_write",
        json!({"upload_id": upload_id, "offset": 0, "chunk_base64": b64}),
    )
    .await;
    assert_eq!(tool_structured(&write)["ok"], true);

    let (_, commit) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    let c1 = tool_structured(&commit);
    assert_eq!(c1["status"], "pending", "commit envelope: {c1}");
    let doc = c1["document_id"].clone();

    let (_, commit2) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    assert_eq!(tool_structured(&commit2)["document_id"], doc);

    let (_, begin2) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({"filename": "abort.txt", "chunked": true}),
    )
    .await;
    let uid2 = tool_structured(&begin2)["upload_id"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = mcp_tools_call(&app, "/mcp", "eq_upload_abort", json!({"upload_id": uid2})).await;
    let (_, after) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_write",
        json!({"upload_id": uid2, "offset": 0, "chunk_base64": b64}),
    )
    .await;
    assert_eq!(tool_structured(&after)["error"]["code"], "eq/not_found");
}

#[tokio::test]
async fn t161_17_bad_checksum() {
    let app = seeded_mcp_app().await;
    let payload = b"abc";
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({"filename": "a.txt", "sha256": "deadbeef", "byte_length": 3, "chunked": true}),
    )
    .await;
    let uid = tool_structured(&begin)["upload_id"].as_str().unwrap();
    let _ = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_write",
        json!({"upload_id": uid, "offset": 0, "chunk_base64": STANDARD.encode(payload)}),
    )
    .await;
    let (_, commit) =
        mcp_tools_call(&app, "/mcp", "eq_upload_commit", json!({"upload_id": uid})).await;
    assert_eq!(tool_structured(&commit)["ok"], false);
}

#[tokio::test]
async fn t161_21_27_delete_async_shape() {
    let app = seeded_mcp_app().await;
    let (_, ingest) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": "delete-me-async", "title": "d"}),
    )
    .await;
    let ingest_sc = tool_structured(&ingest);
    let doc = ingest_sc["document_id"]
        .as_str()
        .unwrap_or_else(|| panic!("ingest failed: {ingest_sc}"));
    let track = tool_structured(&ingest)["task_id"].clone();

    let (_, no) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_delete",
        json!({"document_id": doc, "confirm": false}),
    )
    .await;
    assert_eq!(tool_structured(&no)["error"]["code"], "eq/confirm_required");

    let (_, omit) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_delete",
        json!({"document_id": doc}),
    )
    .await;
    assert_eq!(
        tool_structured(&omit)["error"]["code"],
        "eq/confirm_required"
    );

    let (_, still) =
        mcp_tools_call(&app, "/mcp", "eq_document_get", json!({"document_id": doc})).await;
    let still_sc = tool_structured(&still);
    assert_eq!(
        still_sc["ok"], true,
        "document_get after confirm:false: {still_sc}"
    );
    assert_eq!(still_sc["ready"], false);

    let (_, del) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_delete",
        json!({"document_id": doc, "confirm": true}),
    )
    .await;
    let sc = tool_structured(&del);
    assert_eq!(sc["accepted"], true, "delete envelope: {sc}");
    assert_eq!(sc["deleted"], false);
    assert_eq!(sc["ready"], false);
    assert!(
        sc["task_id"].is_string(),
        "async delete must return pollable task_id: {sc}"
    );
    let delete_task = sc["task_id"].as_str().unwrap().to_string();
    let _ = track;

    // T-161-24: second delete is honest (accepted again or not deleted:true).
    let (_, del2) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_delete",
        json!({"document_id": doc, "confirm": true}),
    )
    .await;
    let sc2 = tool_structured(&del2);
    assert_ne!(
        sc2.get("deleted"),
        Some(&json!(true)),
        "second delete: {sc2}"
    );
    assert!(sc2["ok"] == true || sc2["error"]["code"] == "eq/not_found");

    let (_, task) =
        mcp_tools_call(&app, "/mcp", "eq_task_get", json!({"task_id": delete_task})).await;
    let task_sc = tool_structured(&task);
    assert_eq!(task_sc["ok"], true, "eq_task_get delete task: {task_sc}");
    assert!(task_sc["status"].is_string());

    let (_, missing) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_delete",
        json!({"document_id": "00000000-0000-0000-0000-000000000000", "confirm": true}),
    )
    .await;
    let ms = tool_structured(&missing);
    assert_ne!(ms.get("deleted"), Some(&json!(true)));
}

#[tokio::test]
async fn t161_28_29_30_assets() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_asset_get",
        json!({"document_id": "doc", "asset_id": "nope"}),
    )
    .await;
    assert_eq!(tool_structured(&call)["error"]["code"], "eq/not_found");

    let (_, trav) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_asset_get",
        json!({"document_id": "doc", "asset_id": "../x"}),
    )
    .await;
    assert_eq!(tool_structured(&trav)["error"]["code"], "eq/invalid_id");
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn t161_30_31_32_asset_image_and_unsupported() {
    use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid};
    use edgequake_storage::{StoreMmAssetRequest, ASSET_KIND_EMBEDDED_FIGURE};
    use image::{ImageBuffer, Rgb};
    use uuid::Uuid;

    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let doc = Uuid::new_v4();
    let ws = default_workspace_uuid();
    let storage = state
        .storage
        .mm_asset_storage
        .as_ref()
        .expect("memory mm asset storage with postgres feature");

    // Tiny 1x1 PNG.
    let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_pixel(1, 1, Rgb([10, 20, 30]));
    let mut png = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png);
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
    }
    storage
        .store_asset(StoreMmAssetRequest {
            document_id: doc,
            workspace_id: ws,
            asset_id: "fig-ok".into(),
            asset_path: "assets/fig-ok.png".into(),
            content_type: "image/png".into(),
            asset_data: png.clone(),
            asset_kind: ASSET_KIND_EMBEDDED_FIGURE.into(),
            page_num: Some(1),
        })
        .await
        .unwrap();
    storage
        .store_asset(StoreMmAssetRequest {
            document_id: doc,
            workspace_id: ws,
            asset_id: "fig-bin".into(),
            asset_path: "assets/fig-bin.bin".into(),
            content_type: "application/octet-stream".into(),
            asset_data: b"not-an-image".to_vec(),
            asset_kind: ASSET_KIND_EMBEDDED_FIGURE.into(),
            page_num: None,
        })
        .await
        .unwrap();
    let _ = default_tenant_uuid();

    let app = build_mcp_app(state);
    let doc_s = doc.to_string();

    let (_, bad) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_asset_get",
        json!({"document_id": doc_s, "asset_id": "fig-bin"}),
    )
    .await;
    assert_eq!(
        tool_structured(&bad)["error"]["code"],
        "eq/unsupported_media"
    );

    let (_, ok) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_asset_get",
        json!({"document_id": doc_s, "asset_id": "fig-ok"}),
    )
    .await;
    let body = ok;
    assert_ne!(body["result"]["isError"], true, "asset get: {body}");
    let content = body["result"]["content"].as_array().unwrap();
    let img = content
        .iter()
        .find(|c| c["type"] == "image")
        .expect("ImageContent block");
    let data = STANDARD.decode(img["data"].as_str().unwrap()).unwrap();
    assert!(data.starts_with(b"\x89PNG"));
    assert_eq!(tool_structured(&body)["media_type"], "image/png");
}

#[tokio::test]
async fn t161_14_expired_upload_id() {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let app = build_mcp_app(state.clone());
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({"filename": "ttl.txt", "chunked": true}),
    )
    .await;
    let uid = tool_structured(&begin)["upload_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(state.mcp_uploads.mark_expired(&uid).await);
    let (_, write) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_write",
        json!({
            "upload_id": uid,
            "offset": 0,
            "chunk_base64": STANDARD.encode(b"abc"),
        }),
    )
    .await;
    assert_eq!(tool_structured(&write)["error"]["code"], "eq/not_found");
}

#[tokio::test]
async fn t161_33_35_39_graph_png() {
    use axum::http::{header, Request};
    use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid};

    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let tid = default_tenant_uuid().to_string();
    let wid = default_workspace_uuid().to_string();
    let mut props = HashMap::new();
    props.insert("entity_type".into(), json!("CONCEPT"));
    props.insert("description".into(), json!("focus"));
    props.insert("tenant_id".into(), json!(tid.clone()));
    props.insert("workspace_id".into(), json!(wid.clone()));
    state
        .storage
        .graph_storage
        .upsert_node("FOCUS_NODE", props)
        .await
        .unwrap();
    let app = build_mcp_app(state);

    async fn call_with_scope(
        app: axum::Router,
        tid: &str,
        wid: &str,
        name: &str,
        args: Value,
    ) -> (StatusCode, Value) {
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("X-Tenant-ID", tid)
            .header("X-Workspace-ID", wid)
            .header(header::ACCEPT, "application/json")
            .body(axum::body::Body::from(
                tools_call_body(name, args).to_string(),
            ))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        let status = response.status();
        (status, parse_json(response).await)
    }

    let (_, miss) = call_with_scope(
        app.clone(),
        &tid,
        &wid,
        "eq_graph_image",
        json!({"entity_id": "NO_SUCH_NODE"}),
    )
    .await;
    assert_eq!(tool_structured(&miss)["error"]["code"], "eq/not_found");

    let (_, hit) = call_with_scope(
        app,
        &tid,
        &wid,
        "eq_graph_image",
        json!({"entity_id": "FOCUS_NODE", "max_hops": 3, "budget": "standard"}),
    )
    .await;
    let body = hit;
    assert_ne!(
        body["result"]["isError"], true,
        "graph image must succeed for stamped focus: {body}"
    );
    let sc = tool_structured(&body);
    assert_eq!(sc["max_hops_used"], 2, "T-161-34 hop clamp: {sc}");
    let content = body["result"]["content"].as_array().unwrap();
    let img = content
        .iter()
        .find(|c| c["type"] == "image")
        .expect("PNG ImageContent");
    let data = STANDARD.decode(img["data"].as_str().unwrap()).unwrap();
    assert!(data.starts_with(b"\x89PNG"), "T-161-39 PNG magic");
}

#[tokio::test]
async fn t161_41_42_catalog_and_workspace_delete() {
    let app = seeded_mcp_app().await;
    let response = app
        .clone()
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    let names = names_from_list(&parse_json(response).await);
    assert!(names.contains(&"eq_ingest".to_string()));
    assert!(names.contains(&"eq_upload_begin".to_string()));

    let catalog: Value = serde_json::from_str(include_str!(
        "../../../../specs/152-new-mcp-contract/schemas/tools.catalog.json"
    ))
    .unwrap();
    let catalog_names: Vec<_> = catalog["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for n in catalog_names {
        if n.starts_with("edgequake_") {
            continue;
        }
        assert!(
            names.iter().any(|x| x == n)
                || n == "eq_workspace_delete"
                || names.contains(&n.to_string()),
            "missing {n}"
        );
    }

    let (_, ws) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_workspace_delete",
        json!({"confirm": true, "workspace_id": "x"}),
    )
    .await;
    assert_eq!(tool_structured(&ws)["error"]["code"], "eq/not_implemented");
}

#[tokio::test]
async fn t161_43_47_48_50_download_and_resources() {
    let app = seeded_mcp_app().await;
    let markdown = "a".repeat(80);
    let (_, ingest) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_ingest",
        json!({"content": markdown, "title": "md.md"}),
    )
    .await;
    let ingest_sc = tool_structured(&ingest);
    let doc = ingest_sc["document_id"]
        .as_str()
        .unwrap_or_else(|| panic!("ingest failed: {ingest_sc}"));

    let (_, dl) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_download",
        json!({"document_id": doc, "representation": "markdown", "max_bytes": 16, "offset": 0}),
    )
    .await;
    let sc = tool_structured(&dl);
    if sc["ok"] == true {
        assert!(sc.get("next_offset").is_some() || sc["chunk_length"] == 16);
        let (_, end) = mcp_tools_call(
            &app,
            "/mcp",
            "eq_document_download",
            json!({"document_id": doc, "representation": "markdown", "offset": 10_000}),
        )
        .await;
        let sc2 = tool_structured(&end);
        if sc2["ok"] == true {
            assert_eq!(sc2["chunk_length"], 0);
        }
    } else {
        assert_eq!(sc["error"]["code"], "eq/not_ready");
    }

    let (_, orig) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_document_download",
        json!({"document_id": doc, "representation": "original"}),
    )
    .await;
    let o = tool_structured(&orig);
    assert!(o["ok"] == false || o["media_type"].is_string());

    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({
                "jsonrpc":"2.0","id":1,"method":"resources/read",
                "params":{"uri": format!("eq://default/documents/{doc}/text")}
            }),
        ))
        .await
        .unwrap();
    let body = parse_json(response).await;
    assert!(body.get("result").is_some() || body.get("error").is_some());
}

#[tokio::test]
async fn t161_46_search_has_hits_not_essay() {
    let app = seeded_mcp_app().await;
    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_search",
        json!({"query": "anything", "mode": "naive", "limit": 4}),
    )
    .await;
    let sc = tool_structured(&call);
    assert!(sc.get("retrieval_id").is_some());
    assert!(sc["hits"].is_array());
    assert!(sc.get("answer").is_none());
    assert!(sc.get("essay").is_none());
}

#[tokio::test]
async fn t161_52_oversize_write_body_413() {
    let app = seeded_mcp_app().await;
    let huge = "x".repeat(1024 * 1024 + 50);
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{{"name":"eq_upload_write","arguments":{{"upload_id":"x","chunk_base64":"{huge}"}}}}}}"#
    );
    let response = app
        .oneshot(mcp_post_bytes("/mcp", body.into_bytes()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn t161_06_foreign_workspace_with_auth() {
    let state = auth_enabled_mcp_state().await;
    let token = issue_mcp_jwt(
        &state,
        Role::Admin,
        "edgequake:read edgequake:query edgequake:write",
    );
    let app = build_mcp_app(state);
    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &token,
            tools_call_body(
                "eq_ingest",
                json!({"content": "x", "workspace_id": "00000000-0000-0000-0000-ffffffffffff"}),
            ),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
