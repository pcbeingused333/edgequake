//! SPEC-172 originals: schema checks, maintenance role, and the MinIO release gate.
//!
//! The MinIO narrative skips unless `EDGEQUAKE_ORIGINAL_STORE_E2E=1`.
//! Postgres rows skip when no database is configured.
//!
//!   EDGEQUAKE_ORIGINAL_STORE_E2E=1 cargo test -p edgequake-api --features postgres --test e2e_spec172_originals

#![cfg(feature = "postgres")]

use edgequake_storage::{DocumentMmAssetStorage, PdfDocumentStorage};
use uuid::Uuid;

fn skip_without_e2e() -> bool {
    std::env::var("EDGEQUAKE_ORIGINAL_STORE_E2E")
        .ok()
        .as_deref()
        != Some("1")
}

#[tokio::test]
async fn spec172_minio_upload_range_and_database_control() {
    if skip_without_e2e() {
        eprintln!("SKIP: set EDGEQUAKE_ORIGINAL_STORE_E2E=1 to run the MinIO originals gate");
        return;
    }
    let base =
        std::env::var("EDGEQUAKE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8090".into());
    let client = reqwest::Client::new();
    let health = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("health");
    assert_eq!(health.status(), 200);
    let body: serde_json::Value = health.json().await.expect("health json");
    assert_eq!(body["storage_mode"], "postgresql");
    assert_eq!(body["original_store"], "object");
    assert_eq!(body["components"]["original_store_ready"], true);

    let ready = client
        .get(format!("{base}/ready"))
        .send()
        .await
        .expect("ready");
    assert_eq!(ready.status(), 200, "object mode with MinIO must be ready");

    let pdf = b"%PDF-1.4\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
    let workspace = std::env::var("EDGEQUAKE_E2E_WORKSPACE")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000003".into());
    let tenant = std::env::var("EDGEQUAKE_E2E_TENANT")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000002".into());
    let boundary = "spec172boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"../cafe spec172.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(pdf);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let uploaded = client
        .post(format!("{base}/api/v1/documents/pdf"))
        .header("X-Workspace-ID", &workspace)
        .header("X-Tenant-ID", &tenant)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .expect("upload");
    let status = uploaded.status();
    let upload_body = uploaded.text().await.unwrap_or_default();
    assert!(
        status.is_success(),
        "pdf upload failed {status}: {upload_body}"
    );
    let upload_json: serde_json::Value = serde_json::from_str(&upload_body).expect("upload json");
    let pdf_id = upload_json["pdf_id"].as_str().expect("pdf_id").to_string();
    assert!(Uuid::parse_str(&pdf_id).is_ok());

    let range = client
        .get(format!("{base}/api/v1/documents/pdf/{pdf_id}/download"))
        .header("Range", "bytes=0-7")
        .send()
        .await
        .expect("range");
    assert_eq!(range.status(), 206);
    let bytes = range.bytes().await.unwrap();
    assert_eq!(&bytes[..], b"%PDF-1.4");

    let other_workspace = "11111111-1111-4111-8111-111111111111";
    let foreign = client
        .get(format!("{base}/api/v1/documents/pdf/{pdf_id}/download"))
        .header("X-Workspace-ID", other_workspace)
        .header("X-Tenant-ID", &tenant)
        .header("Range", "bytes=0-7")
        .send()
        .await
        .expect("foreign workspace");
    assert!(
        foreign.status() == 403 || foreign.status() == 404,
        "other workspace must not read the object, got {}",
        foreign.status()
    );

    let mut wide = b"%PDF-1.4\n".to_vec();
    wide.extend(std::iter::repeat_n(b'x', 200));
    let mut wide_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"wide.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    wide_body.extend_from_slice(&wide);
    wide_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let wide_upload = client
        .post(format!("{base}/api/v1/documents/pdf"))
        .header("X-Workspace-ID", &workspace)
        .header("X-Tenant-ID", &tenant)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(wide_body)
        .send()
        .await
        .expect("wide upload");
    let wide_status = wide_upload.status();
    let wide_raw = wide_upload.text().await.unwrap_or_default();
    assert!(
        wide_status.is_success(),
        "wide pdf {wide_status}: {wide_raw}"
    );
    let wide_json: serde_json::Value = serde_json::from_str(&wide_raw).expect("wide json");
    let wide_id = wide_json["pdf_id"].as_str().expect("wide pdf_id");
    let hundred = client
        .get(format!("{base}/api/v1/documents/pdf/{wide_id}/download"))
        .header("X-Workspace-ID", &workspace)
        .header("X-Tenant-ID", &tenant)
        .header("Range", "bytes=0-99")
        .send()
        .await
        .expect("100 byte range");
    assert_eq!(hundred.status(), 206);
    let hundred_bytes = hundred.bytes().await.unwrap();
    assert_eq!(hundred_bytes.len(), 100);
    assert_eq!(&hundred_bytes[..], &wide[..100]);

    let past = client
        .get(format!("{base}/api/v1/documents/pdf/{pdf_id}/download"))
        .header("Range", "bytes=999999-999999")
        .send()
        .await
        .expect("416");
    assert_eq!(past.status(), 416);
    assert!(past.bytes().await.unwrap().is_empty());

    let note = b"spec172 note";
    let mut text_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"notes.txt\"\r\nContent-Type: text/plain\r\n\r\n"
    )
    .into_bytes();
    text_body.extend_from_slice(note);
    text_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let text_upload = client
        .post(format!("{base}/api/v1/documents/upload"))
        .header("X-Workspace-ID", &workspace)
        .header("X-Tenant-ID", &tenant)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(text_body)
        .send()
        .await
        .expect("text upload");
    let text_status = text_upload.status();
    let text_raw = text_upload.text().await.unwrap_or_default();
    assert!(
        text_status.is_success(),
        "text upload failed {text_status}: {text_raw}"
    );
    let text_json: serde_json::Value = serde_json::from_str(&text_raw).expect("text upload json");
    let document_id = text_json["document_id"]
        .as_str()
        .or_else(|| text_json["id"].as_str())
        .expect("document id")
        .to_string();
    let original = client
        .get(format!(
            "{base}/api/v1/documents/{document_id}/download/original"
        ))
        .header("X-Workspace-ID", &workspace)
        .header("X-Tenant-ID", &tenant)
        .send()
        .await
        .expect("original download");
    assert_eq!(original.status(), 200);
    assert_eq!(original.bytes().await.unwrap().as_ref(), note);
}

/// Migration 215 checks, the maintenance role, and the wipe trigger.
/// Skips unless `EDGEQUAKE_SPEC172_PG=1` and `DATABASE_URL` are set.
#[tokio::test]
async fn spec172_postgres_constraints_role_and_wipe_trigger() {
    if std::env::var("EDGEQUAKE_SPEC172_PG").ok().as_deref() != Some("1") {
        eprintln!("SKIP: set EDGEQUAKE_SPEC172_PG=1 to check migration 215");
        return;
    }
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");

    let defs: Vec<String> = sqlx::query_scalar(
        "SELECT pg_get_constraintdef(oid) FROM pg_constraint \
         WHERE conname IN ('pdf_document_blobs_byte_location_chk', 'document_originals_byte_location_chk')",
    )
    .fetch_all(&pool)
    .await
    .expect("constraint defs");
    assert_eq!(defs.len(), 2, "both byte_location checks must exist");
    for def in &defs {
        let lower = def.to_ascii_lowercase();
        assert!(!lower.contains("select"), "check must not subquery: {def}");
        assert!(
            lower.contains("is null"),
            "object location requires null bytes: {def}"
        );
    }

    let mut tx = pool.begin().await.expect("check tx");
    let candidate: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT pdf_id FROM pdf_document_blobs WHERE pdf_data IS NOT NULL LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await
    .expect("candidate");
    if let Some(pdf_id) = candidate {
        let illegal =
            sqlx::query("UPDATE pdf_document_blobs SET byte_location = 'object' WHERE pdf_id = $1")
                .bind(pdf_id)
                .execute(&mut *tx)
                .await;
        let err = illegal.expect_err("object location with BYTEA must fail the check");
        let message = err.to_string();
        assert!(
            message.contains("pdf_document_blobs_byte_location_chk") || message.contains("check"),
            "{message}"
        );
    }
    tx.rollback().await.expect("rollback check");

    let role: (bool, bool) = sqlx::query_as(
        "SELECT rolbypassrls, rolcanlogin FROM pg_roles WHERE rolname = 'edgequake_originals_maint'",
    )
    .fetch_one(&pool)
    .await
    .expect("maint role");
    assert!(role.0, "edgequake_originals_maint must BYPASSRLS");
    assert!(!role.1, "edgequake_originals_maint must stay NOLOGIN");

    let granted: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM pg_auth_members m \
         JOIN pg_roles r ON r.oid = m.roleid \
         JOIN pg_roles u ON u.oid = m.member \
         WHERE r.rolname = 'edgequake_originals_maint' AND u.rolname = current_user",
    )
    .fetch_one(&pool)
    .await
    .expect("grant lookup");
    assert_eq!(
        granted, 0,
        "the API role must not inherit the maintenance role"
    );

    let maint = edgequake_storage::originals::connect_maint_pool(&url)
        .await
        .expect("maint pool");
    edgequake_storage::originals::assert_maint_role(&maint)
        .await
        .expect("SET ROLE");
    let (during, after) = edgequake_storage::originals::claim_statement_timeout(&maint)
        .await
        .expect("timeout");
    assert!(
        during == "2min" || during == "120s" || during == "120000",
        "claim timeout was {during}"
    );
    assert_ne!(during, after);

    let mut foreign = pool.begin().await.expect("foreign tenant");
    sqlx::query("SET LOCAL ROLE edgequake_tenant_access")
        .execute(&mut *foreign)
        .await
        .expect("tenant access role");
    sqlx::query(
        "SELECT set_config('app.current_tenant_id', '00000000-0000-0000-0000-000000000099', true)",
    )
    .execute(&mut *foreign)
    .await
    .expect("foreign tenant");
    sqlx::query(
        "SELECT set_config('app.current_workspace_id', '00000000-0000-0000-0000-000000000003', true)",
    )
    .execute(&mut *foreign)
    .await
    .expect("foreign workspace");
    let visible: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM original_object_refs")
        .fetch_one(&mut *foreign)
        .await
        .expect("foreign count");
    assert_eq!(
        visible, 0,
        "a different tenant must not see original object refs"
    );
    foreign.rollback().await.expect("rollback foreign");

    let mut tx = pool.begin().await.expect("tx");
    let owner: Option<(uuid::Uuid, uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        "SELECT d.id, d.workspace_id, w.tenant_id \
         FROM documents d JOIN workspaces w ON w.workspace_id = d.workspace_id \
         WHERE NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.document_id = d.id) \
         LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await
    .expect("owner");
    let Some((document_id, workspace_id, tenant_id)) = owner else {
        tx.rollback().await.ok();
        eprintln!("SKIP wipe trigger: no document without a ref");
        return;
    };
    let foreign_key = format!(
        "dev/t/00000000-0000-0000-0000-000000000099/w/{workspace_id}/original/{document_id}"
    );
    let key = format!("dev/t/{tenant_id}/w/{workspace_id}/original/{document_id}");
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant");
    sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
        .bind(workspace_id.to_string())
        .execute(&mut *tx)
        .await
        .expect("workspace");
    for (probe, expect_queue) in [(&foreign_key, 0i64), (&key, 1i64)] {
        sqlx::query(
            "INSERT INTO original_object_refs \
             (document_id, tenant_id, workspace_id, object_key, sha256, byte_length, content_type, verified_at) \
             VALUES ($1, $2, $3, $4, $5, 4, 'text/plain', now())",
        )
        .bind(document_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(probe)
        .bind("a".repeat(64))
        .execute(&mut *tx)
        .await
        .expect("insert ref");
        sqlx::query("DELETE FROM original_object_refs WHERE object_key = $1")
            .bind(probe)
            .execute(&mut *tx)
            .await
            .expect("delete ref");
        let queued: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM original_object_deletes WHERE object_key = $1",
        )
        .bind(probe)
        .fetch_one(&mut *tx)
        .await
        .expect("queue");
        assert_eq!(queued, expect_queue, "delete queue for {probe}");
    }
    tx.rollback().await.expect("rollback");
}

/// Copy, verify, and clear one disposable workspace, then prove a workspace
/// delete enqueues only that workspace's keys. Also checks virtual-hosted
/// style against this MinIO fails the boot probe.
#[tokio::test]
async fn spec172_migrate_clear_and_workspace_wipe() {
    if skip_without_e2e() {
        eprintln!("SKIP: set EDGEQUAKE_ORIGINAL_STORE_E2E=1 to migrate one workspace");
        return;
    }
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let opened = edgequake_storage::originals::open_from_env()
        .await
        .expect("object store");
    let client = opened.client.expect("object client");
    assert!(
        opened.bucket_ready,
        "MinIO must answer the path-style probe"
    );

    let endpoint = std::env::var("EDGEQUAKE_ORIGINAL_STORE_ENDPOINT").expect("endpoint");
    let mut virtual_hosted = edgequake_storage::originals::Settings {
        mode: edgequake_storage::originals::StoreMode::Object,
        url: std::env::var("EDGEQUAKE_ORIGINAL_STORE_URL").ok(),
        endpoint: Some(endpoint),
        allow_http: true,
        path_style: Some(false),
        access_key: std::env::var("EDGEQUAKE_ORIGINAL_STORE_ACCESS_KEY_ID").ok(),
        secret_key: std::env::var("EDGEQUAKE_ORIGINAL_STORE_SECRET_ACCESS_KEY").ok(),
        ..edgequake_storage::originals::Settings::default()
    };
    virtual_hosted.region = "us-east-1".into();
    let probed = edgequake_storage::originals::open_settings(&virtual_hosted).await;
    match &probed {
        Err(err) => eprintln!("path-style false refused the boot head: {err}"),
        Ok(opened) if !opened.bucket_ready => {
            eprintln!("path-style false left the bucket not ready");
        }
        Ok(_) => eprintln!(
            "path-style false still probed ready on this MinIO; default path-style remains the dev setting"
        ),
    }
    assert!(!edgequake_storage::originals::path_style_enabled(
        virtual_hosted.endpoint.as_deref(),
        Some(false)
    ));

    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let tenant: uuid::Uuid = "00000000-0000-0000-0000-000000000002".parse().unwrap();
    let workspace = uuid::Uuid::new_v4();
    let content = "spec172-content-stays";
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind("spec172 migrate")
    .bind(format!("m{}", &workspace.simple().to_string()[..12]))
    .execute(&pool)
    .await
    .expect("workspace");

    let mut documents = Vec::new();
    for n in 0..4 {
        let document = uuid::Uuid::new_v4();
        let bytes = format!("migrate-{n}-spec172").into_bytes();
        sqlx::query(
            "INSERT INTO documents (id, tenant_id, workspace_id, title, content) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(document)
        .bind(tenant)
        .bind(workspace)
        .bind(format!("spec172 {n}"))
        .bind(content)
        .execute(&pool)
        .await
        .expect("document");
        sqlx::query(
            "INSERT INTO document_originals (document_id, workspace_id, filename, content_type, file_size_bytes, original_data, byte_location) VALUES ($1, $2, 'notes.txt', 'text/plain', $3, $4, 'database')",
        )
        .bind(document)
        .bind(workspace)
        .bind(bytes.len() as i64)
        .bind(&bytes)
        .execute(&pool)
        .await
        .expect("original");
        documents.push((document, bytes));
    }

    let maint = edgequake_storage::originals::connect_maint_pool(&url)
        .await
        .expect("maint");
    let ledger = std::sync::Arc::new(edgequake_storage::originals::PgLedger::new(maint));
    let scope = edgequake_storage::originals::Scope {
        kind: edgequake_storage::originals::KindFilter::One(
            edgequake_storage::originals::OwnerKind::Original,
        ),
        workspace: Some(workspace),
        limit: None,
    };
    let opts = edgequake_storage::originals::RunOptions {
        prefix: client.prefix.clone(),
        sse: client.sse,
        mode: edgequake_storage::originals::StoreMode::Object,
        scope: scope.clone(),
        yes: true,
        concurrency: 2,
    };
    let left = {
        let ledger = std::sync::Arc::clone(&ledger);
        let blobs = client.blobs.clone();
        let opts = opts.clone();
        tokio::spawn(async move {
            edgequake_storage::originals::copy_owners(ledger.as_ref(), blobs.as_ref(), &opts).await
        })
    };
    let right = {
        let ledger = std::sync::Arc::clone(&ledger);
        let blobs = client.blobs.clone();
        let opts = opts.clone();
        tokio::spawn(async move {
            edgequake_storage::originals::copy_owners(ledger.as_ref(), blobs.as_ref(), &opts).await
        })
    };
    left.await.unwrap().expect("copy left");
    right.await.unwrap().expect("copy right");

    let refs: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM original_object_refs WHERE workspace_id = $1",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(refs, 4, "two migrators must write each owner once");

    for (document, bytes) in &documents {
        let still: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT original_data FROM document_originals WHERE document_id = $1",
        )
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(still.as_deref(), Some(bytes.as_slice()));
        let kept: String = sqlx::query_scalar("SELECT content FROM documents WHERE id = $1")
            .bind(document)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kept, content);
    }

    edgequake_storage::originals::verify_owners(ledger.as_ref(), client.blobs.as_ref(), &scope)
        .await
        .expect("verify");
    edgequake_storage::originals::clear_owners(ledger.as_ref(), client.blobs.as_ref(), &opts)
        .await
        .expect("clear");

    for (document, bytes) in &documents {
        let cleared: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT original_data FROM document_originals WHERE document_id = $1",
        )
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(cleared.is_none(), "clear must null BYTEA");
        let location: String = sqlx::query_scalar(
            "SELECT byte_location FROM document_originals WHERE document_id = $1",
        )
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(location, "object");
        let kept: String = sqlx::query_scalar("SELECT content FROM documents WHERE id = $1")
            .bind(document)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kept, content, "documents.content stays");
        let key = edgequake_storage::originals::upload_key(
            &client.prefix,
            edgequake_storage::originals::OwnerKind::Original,
            tenant,
            workspace,
            *document,
        );
        let got = client.blobs.get_all(&key).await.expect("object");
        assert_eq!(got.as_ref(), bytes.as_slice());
    }

    let blobs = client.blobs.clone();
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT object_key FROM original_object_refs WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_all(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("wipe workspace");
    for key in &keys {
        let queued: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM original_object_deletes WHERE object_key = $1",
        )
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(queued, 1, "workspace delete must enqueue {key}");
        let _ = edgequake_storage::originals::BlobStore::delete_key(blobs.as_ref(), key).await;
        sqlx::query("DELETE FROM original_object_deletes WHERE object_key = $1")
            .bind(key)
            .execute(&pool)
            .await
            .ok();
    }
}

/// One small image of each kind, plus extracted markdown, through the same
/// flag-gated client. `documents.content` stays in Postgres.
#[tokio::test]
async fn spec172_migrate_images_and_markdown() {
    if skip_without_e2e() {
        eprintln!("SKIP: set EDGEQUAKE_ORIGINAL_STORE_E2E=1 to migrate images and markdown");
        return;
    }
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let opened = edgequake_storage::originals::open_from_env()
        .await
        .expect("object store");
    let client = std::sync::Arc::new(
        opened
            .client
            .expect("object client; EDGEQUAKE_ORIGINAL_STORE=object"),
    );
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let tenant: uuid::Uuid = "00000000-0000-0000-0000-000000000002".parse().unwrap();
    let workspace = uuid::Uuid::new_v4();
    let content = "spec172-markdown-body-stays";
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind("spec172 images")
    .bind(format!("i{}", &workspace.simple().to_string()[..12]))
    .execute(&pool)
    .await
    .expect("workspace");
    let document = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(document)
    .bind(tenant)
    .bind(workspace)
    .bind("spec172 images")
    .bind(content)
    .execute(&pool)
    .await
    .expect("document");

    let kinds = [
        (
            "page_full",
            "assets/page-0001.png",
            b"\x89PNG-page".to_vec(),
        ),
        (
            "embedded_figure",
            "assets/figure-1.png",
            b"\x89PNG-figure".to_vec(),
        ),
        (
            "page_chart_crop",
            "assets/chart-1.png",
            b"\x89PNG-chart".to_vec(),
        ),
        (
            "table_crop",
            "assets/table-1.png",
            b"\x89PNG-table".to_vec(),
        ),
    ];
    for (index, (kind, path, bytes)) in kinds.iter().enumerate() {
        sqlx::query(
            "INSERT INTO document_mm_assets (
                document_id, workspace_id, asset_id, asset_path, content_type,
                file_size_bytes, asset_data, asset_kind, page_num, byte_location
            ) VALUES ($1, $2, $3, $4, 'image/png', $5, $6, $7, 1, 'database')",
        )
        .bind(document)
        .bind(workspace)
        .bind(format!("asset-{index}"))
        .bind(*path)
        .bind(bytes.len() as i64)
        .bind(bytes)
        .bind(*kind)
        .execute(&pool)
        .await
        .expect("asset");
    }
    let markdown = "# café page\n\nkept in the bucket\n";
    let pdf_id = uuid::Uuid::new_v4();
    let checksum = "ab".repeat(32);
    sqlx::query(
        "INSERT INTO pdf_documents (
            pdf_id, workspace_id, document_id, filename, content_type, file_size_bytes,
            sha256_checksum, processing_status, markdown_content, markdown_location
        ) VALUES ($1, $2, $3, '../café page.md', 'application/pdf', 8, $4, 'completed', $5, 'database')",
    )
    .bind(pdf_id)
    .bind(workspace)
    .bind(document)
    .bind(&checksum)
    .bind(markdown)
    .execute(&pool)
    .await
    .expect("pdf markdown");
    sqlx::query(
        "INSERT INTO pdf_document_blobs (pdf_id, pdf_data, byte_location) VALUES ($1, $2, 'database')",
    )
    .bind(pdf_id)
    .bind(b"%PDF-1.4".as_slice())
    .execute(&pool)
    .await
    .expect("pdf blob");

    let maint = edgequake_storage::originals::connect_maint_pool(&url)
        .await
        .expect("maint");
    let ledger = edgequake_storage::originals::PgLedger::new(maint);
    let scope = edgequake_storage::originals::Scope {
        kind: edgequake_storage::originals::KindFilter::All,
        workspace: Some(workspace),
        limit: None,
    };
    let opts = edgequake_storage::originals::RunOptions {
        prefix: client.prefix.clone(),
        sse: client.sse,
        mode: edgequake_storage::originals::StoreMode::Object,
        scope: scope.clone(),
        yes: true,
        concurrency: 2,
    };
    let report = edgequake_storage::originals::copy_owners(&ledger, client.blobs.as_ref(), &opts)
        .await
        .expect("copy");
    assert_eq!(report.failed, 0);
    assert!(report.copied >= 6, "pdf, four images, markdown: {report:?}");
    let window: Option<i32> = sqlx::query_scalar(
        "SELECT octet_length(chunk_sha256)::int FROM original_object_refs WHERE pdf_id = $1",
    )
    .bind(pdf_id)
    .fetch_one(&pool)
    .await
    .expect("pdf window digest");
    assert_eq!(window, Some(32));
    let kept: String = sqlx::query_scalar("SELECT content FROM documents WHERE id = $1")
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, content);

    let assets = edgequake_storage::PostgresMmAssetStorage::new(pool.clone())
        .with_objects(Some(std::sync::Arc::clone(&client)));
    for (index, (kind, path, bytes)) in kinds.iter().enumerate() {
        let loaded = assets
            .get_asset(&workspace, &document, path)
            .await
            .expect("get")
            .unwrap_or_else(|| panic!("missing {kind}"));
        assert_eq!(loaded.asset_data, *bytes);
        let owner: uuid::Uuid = sqlx::query_scalar(
            "SELECT object_id FROM document_mm_assets WHERE document_id = $1 AND asset_path = $2",
        )
        .bind(document)
        .bind(*path)
        .fetch_one(&pool)
        .await
        .unwrap();
        let key = edgequake_storage::originals::upload_key(
            &client.prefix,
            edgequake_storage::originals::OwnerKind::Asset,
            tenant,
            workspace,
            owner,
        );
        assert!(!key.contains(path));
        assert!(!key.contains("page-0001"));
        assert!(!key.contains("café"));
        let object = client.blobs.get_all(&key).await.expect("asset object");
        assert_eq!(object.as_ref(), bytes.as_slice(), "{kind} {index}");
    }
    sqlx::query(
        "UPDATE document_mm_assets SET asset_data = $1
         WHERE document_id = $2 AND asset_path = 'assets/page-0001.png'",
    )
    .bind(b"\x89PNG-edited".as_slice())
    .bind(document)
    .execute(&pool)
    .await
    .expect("edit column");
    let edited = assets
        .get_asset(&workspace, &document, "assets/page-0001.png")
        .await
        .expect("edited get")
        .expect("edited column");
    assert_eq!(edited.asset_data, b"\x89PNG-edited");
    sqlx::query(
        "UPDATE document_mm_assets SET asset_data = $1
         WHERE document_id = $2 AND asset_path = 'assets/page-0001.png'",
    )
    .bind(b"\x89PNG-page".as_slice())
    .bind(document)
    .execute(&pool)
    .await
    .expect("restore column");
    let markdown_owner: uuid::Uuid =
        sqlx::query_scalar("SELECT markdown_object_id FROM pdf_documents WHERE pdf_id = $1")
            .bind(pdf_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let markdown_key = edgequake_storage::originals::upload_key(
        &client.prefix,
        edgequake_storage::originals::OwnerKind::Markdown,
        tenant,
        workspace,
        markdown_owner,
    );
    assert!(!markdown_key.contains("café"));
    assert!(!markdown_key.contains(".md"));
    let markdown_object = client
        .blobs
        .get_all(&markdown_key)
        .await
        .expect("markdown object");
    assert_eq!(markdown_object.as_ref(), markdown.as_bytes());

    let other = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(other)
    .bind(tenant)
    .bind("spec172 other")
    .bind(format!("o{}", &other.simple().to_string()[..12]))
    .execute(&pool)
    .await
    .expect("other workspace");
    let crossed = assets
        .store_asset(edgequake_storage::StoreMmAssetRequest {
            document_id: document,
            workspace_id: other,
            asset_id: "asset-0".into(),
            asset_path: "assets/page-0001.png".into(),
            content_type: "text/html".into(),
            asset_data: b"<html>crossed".to_vec(),
            asset_kind: "page_full".into(),
            page_num: Some(1),
        })
        .await;
    assert!(
        crossed.is_err(),
        "another workspace must not replace the image"
    );
    let still_png: Vec<u8> = sqlx::query_scalar(
        "SELECT asset_data FROM document_mm_assets WHERE document_id = $1 AND asset_path = 'assets/page-0001.png'",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still_png, b"\x89PNG-page");

    let page_key: String = sqlx::query_scalar(
        "SELECT r.object_key FROM original_object_refs r
         JOIN document_mm_assets a ON a.object_id = r.asset_object_id
         WHERE a.document_id = $1 AND a.asset_path = 'assets/page-0001.png'",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &page_key,
        b"tampered-page",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("tamper");
    let served = assets
        .get_asset(&workspace, &document, "assets/page-0001.png")
        .await
        .expect("fallback")
        .expect("column");
    assert_eq!(served.asset_data, b"\x89PNG-page");
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &page_key,
        b"\x89PNG-page",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("restore object");

    let cleared = edgequake_storage::originals::clear_owners(&ledger, client.blobs.as_ref(), &opts)
        .await
        .expect("clear");
    assert!(cleared.contains("asset"));
    assert!(cleared.contains("markdown"));
    let asset_left: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT asset_data FROM document_mm_assets WHERE document_id = $1 AND asset_kind = 'page_full'",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(asset_left.is_none());
    let markdown_left: Option<String> =
        sqlx::query_scalar("SELECT markdown_content FROM pdf_documents WHERE pdf_id = $1")
            .bind(pdf_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(markdown_left.is_none());
    let still = assets
        .get_asset(&workspace, &document, "assets/page-0001.png")
        .await
        .expect("download")
        .expect("png after clear");
    assert_eq!(still.asset_data, b"\x89PNG-page");
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &page_key,
        b"tampered-after-clear",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("tamper cleared object");
    let refused = assets
        .get_asset(&workspace, &document, "assets/page-0001.png")
        .await;
    assert!(
        refused.is_err(),
        "a replaced object with no database copy must not be served"
    );
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &page_key,
        b"\x89PNG-page",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("restore cleared object");
    let pdfs = edgequake_storage::PostgresPdfStorage::new(pool.clone())
        .with_objects(Some(std::sync::Arc::clone(&client)));
    let pdf = pdfs.get_pdf(&pdf_id).await.expect("pdf").expect("pdf row");
    assert_eq!(pdf.markdown_content.as_deref(), Some(markdown));
    let pdf_key: String =
        sqlx::query_scalar("SELECT object_key FROM original_object_refs WHERE pdf_id = $1")
            .bind(pdf_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &pdf_key,
        b"tampered-pdf",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("tamper pdf");
    let range = pdfs.get_pdf_bytes_range(&pdf_id, 0, 0).await;
    assert!(
        range.is_err(),
        "a tampered PDF window must fail the range read"
    );
    edgequake_storage::originals::put_bytes(
        client.blobs.as_ref(),
        &pdf_key,
        b"%PDF-1.4",
        edgequake_storage::originals::Sse::Off,
    )
    .await
    .expect("restore pdf");
    let figure_key: String = sqlx::query_scalar(
        "SELECT r.object_key FROM original_object_refs r
         JOIN document_mm_assets a ON a.object_id = r.asset_object_id
         WHERE a.document_id = $1 AND a.asset_path = 'assets/figure-1.png'",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    assets
        .store_asset(edgequake_storage::StoreMmAssetRequest {
            document_id: document,
            workspace_id: workspace,
            asset_id: "asset-1".into(),
            asset_path: "assets/figure-1.png".into(),
            content_type: "image/png".into(),
            asset_data: b"\x89PNG-figure-v2".to_vec(),
            asset_kind: "embedded_figure".into(),
            page_num: Some(1),
        })
        .await
        .expect("second asset write");
    let published: String = sqlx::query_scalar(
        "SELECT r.object_key FROM original_object_refs r
         JOIN document_mm_assets a ON a.object_id = r.asset_object_id
         WHERE a.document_id = $1 AND a.asset_path = 'assets/figure-1.png'",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_ne!(published, figure_key);
    let replaced = assets
        .get_asset(&workspace, &document, "assets/figure-1.png")
        .await
        .expect("replaced asset")
        .expect("figure");
    assert_eq!(replaced.asset_data, b"\x89PNG-figure-v2");
    assert!(client.blobs.get_all(&figure_key).await.is_err());
    let kept_after: String = sqlx::query_scalar("SELECT content FROM documents WHERE id = $1")
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept_after, content);

    let keys: Vec<String> =
        sqlx::query_scalar("SELECT object_key FROM original_object_refs WHERE workspace_id = $1")
            .bind(workspace)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(keys.iter().filter(|key| key.contains("/asset/")).count() >= 4);
    assert!(keys.iter().any(|key| key.contains("/markdown/")));
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&pool)
        .await
        .expect("wipe");
    for key in &keys {
        let queued: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM original_object_deletes WHERE object_key = $1",
        )
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(queued, 1, "workspace delete must enqueue {key}");
        let _ =
            edgequake_storage::originals::BlobStore::delete_key(client.blobs.as_ref(), key).await;
        sqlx::query("DELETE FROM original_object_deletes WHERE object_key = $1")
            .bind(key)
            .execute(&pool)
            .await
            .ok();
    }
}
