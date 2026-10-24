//! Document clearance for one request (SPEC-174).
//!
//! `bind_request` is the only caller that loads rules. Handlers read the
//! scope it stored. SQL writes go through this module so a handler cannot
//! update `classification_rank` on its own.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use axum::extract::Request;
use axum::response::IntoResponse;
use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_core::clearance::{
    allows, allows_relabel, compile_subject, guc_compartment_ids, rank_from_name, rank_name,
    ClearanceAction, ClearanceRule, Effect, MatchKind, ObjectLabel, RoleGrant, VisibilityScope,
    RANK_INTERNAL, RANK_UNVERIFIED,
};
use edgequake_storage::clearance_session::ClearanceSession;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::RequestAuthContext;
use crate::handlers::documents_types::CompartmentView;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Process flag. Read once. Unset means off.
pub fn break_glass_enabled() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| {
        matches!(
            std::env::var("EDGEQUAKE_CLEARANCE_BREAK_GLASS").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        )
    })
}

/// Load the caller's scope and store it on the request.
///
/// No pool means enforcement off. That is the path unit tests and a binary
/// built without Postgres take. A pool error fails the request.
pub async fn attach_clearance(
    state: &AppState,
    request: &mut Request,
    auth: &RequestAuthContext,
    ctx: &TenantContext,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let workspace = ctx
            .workspace_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok());
        let user_id = Uuid::parse_str(&auth.user_id).ok();
        let (scope, session) = if let Some(workspace_id) = workspace {
            load_scope(pool, workspace_id, user_id).await?
        } else {
            (VisibilityScope::enforcement_off(), ClearanceSession::off())
        };
        if scope.subject.break_glass && scope.subject.enforcement {
            crate::services::audit::record_compliance_event(
                state,
                ctx.tenant_id.clone().unwrap_or_default(),
                AuditEventType::Authorization,
                "clearance.break_glass",
                AuditResult::Warning,
                ctx.workspace_id.clone(),
                Some(auth.user_id.clone()),
                None,
            );
        }
        request.extensions_mut().insert(scope);
        request.extensions_mut().insert(session);
        return Ok(());
    }
    request
        .extensions_mut()
        .insert(VisibilityScope::enforcement_off());
    request.extensions_mut().insert(ClearanceSession::off());
    Ok(())
}

pub fn scope_on(request: &Request) -> VisibilityScope {
    request
        .extensions()
        .get::<VisibilityScope>()
        .cloned()
        .unwrap_or_else(VisibilityScope::enforcement_off)
}

/// Run the rest of the request with the session `bind_request` stored.
pub async fn run_scoped(
    request: Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let session = request
        .extensions()
        .get::<ClearanceSession>()
        .cloned()
        .unwrap_or_else(ClearanceSession::off);
    let path = request.uri().path().to_string();
    edgequake_storage::clearance_session::scope(session, async move {
        if let Some(document_id) = document_id_in_path(&path) {
            match must_hide(document_id).await {
                Ok(true) => {
                    return not_found(document_id).into_response();
                }
                Ok(false) => {}
                Err(error) => return error.into_response(),
            }
        }
        next.run(request).await
    })
    .await
}

/// `/api/v1/documents/{uuid}` and its children. List and upload have no id.
fn document_id_in_path(path: &str) -> Option<&str> {
    let mut parts = path.split('/');
    while let Some(part) = parts.next() {
        if part == "documents" {
            let id = parts.next()?;
            return Uuid::parse_str(id).is_ok().then_some(id);
        }
    }
    None
}

/// `true` when an enforced caller must not see this id. Missing and hidden
/// are the same answer.
pub async fn must_hide(document_id: &str) -> Result<bool, ApiError> {
    let Some(session) = ClearanceSession::current() else {
        return Ok(false);
    };
    if session.predicate_sql().is_empty() {
        return Ok(false);
    }
    let Ok(id) = Uuid::parse_str(document_id) else {
        return Ok(true);
    };
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = crate::services::relational_sidecar_store::sidecar_pool() else {
            return Ok(true);
        };
        return row_hidden(pool.as_ref(), id, &session).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = id;
        Ok(false)
    }
}

/// Same not-found error a missing id already produces.
pub fn not_found(document_id: &str) -> ApiError {
    ApiError::NotFound(format!("Document {document_id} not found"))
}

#[cfg(feature = "postgres")]
async fn row_hidden(
    pool: &sqlx::PgPool,
    id: Uuid,
    session: &ClearanceSession,
) -> Result<bool, ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, session)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    let found: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM public.documents WHERE id = $1 AND (public.clearance_covers(classification_rank, compartment_ids) OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(found.is_none())
}

#[cfg(feature = "postgres")]
pub async fn load_scope(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<(VisibilityScope, ClearanceSession), ApiError> {
    let settings: Option<(bool, i64)> = sqlx::query_as(
        "SELECT enforcement, policy_epoch FROM public.workspace_clearance_settings WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let (enforcement, epoch) = settings.unwrap_or((false, 0));
    let grants = if let Some(user_id) = user_id {
        load_grants(pool, workspace_id, user_id).await?
    } else {
        Vec::new()
    };
    let mut subject = compile_subject(&grants, user_id);
    subject.enforcement = enforcement;
    subject.break_glass = break_glass_enabled();
    let vocabulary = load_vocabulary(pool, workspace_id).await?;
    let scope = VisibilityScope::new(subject, epoch);
    let session = session_from(&scope, &vocabulary);
    Ok((scope, session))
}

#[cfg(feature = "postgres")]
fn session_from(scope: &VisibilityScope, vocabulary: &[Uuid]) -> ClearanceSession {
    ClearanceSession {
        enforced: scope.subject.enforcement,
        break_glass: scope.subject.break_glass,
        system: false,
        steward: scope.subject.steward,
        rank: i16::from(scope.subject.rank),
        compartments: guc_compartment_ids(&scope.subject, vocabulary)
            .into_iter()
            .collect(),
        allow_empty: scope.subject.allow_empty_compartments,
        user_id: scope.subject.user_id,
        policy_epoch: scope.policy_epoch,
        fingerprint: scope.fingerprint.clone(),
    }
}

#[cfg(feature = "postgres")]
async fn load_vocabulary(pool: &sqlx::PgPool, workspace_id: Uuid) -> Result<Vec<Uuid>, ApiError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT tag_id FROM public.clearance_tags WHERE workspace_id = $1 AND kind = 'compartment'",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct RuleRow {
    role_id: Uuid,
    steward: bool,
    effect: Option<String>,
    match_kind: Option<String>,
    rank: Option<i16>,
    tag_ids: Option<Vec<Uuid>>,
}

#[cfg(feature = "postgres")]
async fn load_grants(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<RoleGrant>, ApiError> {
    let rows: Vec<RuleRow> = sqlx::query_as(
        "SELECT r.role_id, r.steward, ru.effect, ru.match_kind, ru.rank, ru.tag_ids
         FROM public.memberships m
         JOIN public.membership_security_roles ms ON ms.membership_id = m.membership_id
         JOIN public.security_roles r ON r.role_id = ms.role_id
         LEFT JOIN public.security_role_rules ru ON ru.role_id = r.role_id
         WHERE m.user_id = $1 AND m.workspace_id = $2 AND m.is_active
         ORDER BY r.role_id, ru.position",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let mut grouped: BTreeMap<Uuid, RoleGrant> = BTreeMap::new();
    for row in rows {
        let grant = grouped.entry(row.role_id).or_insert(RoleGrant {
            steward: row.steward,
            rules: Vec::new(),
        });
        if let Some(rule) = rule_from_row(&row) {
            grant.rules.push(rule);
        }
    }
    Ok(grouped.into_values().collect())
}

#[cfg(feature = "postgres")]
fn rule_from_row(row: &RuleRow) -> Option<ClearanceRule> {
    let effect = match row.effect.as_deref()? {
        "allow" => Effect::Allow,
        "deny" => Effect::Deny,
        _ => return None,
    };
    let kind = match row.match_kind.as_deref()? {
        "classification_at_most" => MatchKind::ClassificationAtMost {
            rank: u8::try_from(row.rank.unwrap_or(0)).unwrap_or(0),
        },
        "any_of" => MatchKind::AnyOf {
            tag_ids: row
                .tag_ids
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        },
        "all_of" => MatchKind::AllOf {
            tag_ids: row
                .tag_ids
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        },
        "untagged" => MatchKind::Untagged,
        _ => return None,
    };
    Some(ClearanceRule { effect, kind })
}

/// Labels for the documents a caller is already allowed to see.
/// Document rank, then one vocabulary tag the document carries (if any).
#[cfg(feature = "postgres")]
type LabelRow = (
    Uuid,
    Option<i16>,
    Option<Uuid>,
    Option<String>,
    Option<String>,
);

#[cfg(feature = "postgres")]
pub async fn labels_for(
    pool: &sqlx::PgPool,
    ids: &[Uuid],
) -> Result<BTreeMap<String, (Option<String>, Vec<CompartmentView>)>, ApiError> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows: Vec<LabelRow> = sqlx::query_as(
        "SELECT d.id, d.classification_rank, t.tag_id, t.key, t.value
         FROM public.documents d
         LEFT JOIN public.document_labels dl ON dl.document_id = d.id
         LEFT JOIN public.clearance_tags t ON t.tag_id = dl.tag_id AND t.kind = 'compartment'
         WHERE d.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let mut out: BTreeMap<String, (Option<String>, Vec<CompartmentView>)> = BTreeMap::new();
    for (id, rank, tag_id, key, value) in rows {
        let entry = out.entry(id.to_string()).or_insert_with(|| {
            (
                rank.and_then(|rank| u8::try_from(rank).ok())
                    .and_then(rank_name)
                    .map(str::to_string),
                Vec::new(),
            )
        });
        if let (Some(tag_id), Some(key), Some(value)) = (tag_id, key, value) {
            if !entry.1.iter().any(|item| item.id == tag_id.to_string()) {
                entry.1.push(CompartmentView {
                    id: tag_id.to_string(),
                    key,
                    value,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(feature = "postgres")]
pub async fn membership_role(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    workspace_id: Uuid,
) -> Result<Option<String>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM public.memberships WHERE user_id = $1 AND workspace_id = $2 AND is_active LIMIT 1",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(row.map(|(role,)| role))
}

/// Seed classifications, backfill ranks, grant steward, then turn enforcement on.
#[cfg(feature = "postgres")]
pub async fn enable(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
    strict_start: bool,
) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.clearance_tags (tag_id, workspace_id, kind, key, value, rank)
         VALUES
           (gen_random_uuid(), $1, 'classification', 'public', 'public', 1),
           (gen_random_uuid(), $1, 'classification', 'internal', 'internal', 2),
           (gen_random_uuid(), $1, 'classification', 'confidential', 'confidential', 3),
           (gen_random_uuid(), $1, 'classification', 'restricted', 'restricted', 4),
           (gen_random_uuid(), $1, 'classification', 'unverified', 'unverified', 5)
         ON CONFLICT (workspace_id, kind, key, value) DO NOTHING",
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let rank: i16 = if strict_start {
        i16::from(RANK_UNVERIFIED)
    } else {
        i16::from(RANK_INTERNAL)
    };
    sqlx::query(
        "UPDATE public.documents
         SET classification_rank = $2, compartment_ids = COALESCE(compartment_ids, '{}')
         WHERE workspace_id = $1 AND classification_rank IS NULL",
    )
    .bind(workspace_id)
    .bind(rank)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    copy_labels(&mut tx, workspace_id).await?;
    let role_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO public.security_roles (role_id, workspace_id, name, steward)
         VALUES (gen_random_uuid(), $1, 'Steward', true)
         ON CONFLICT (workspace_id, name) DO UPDATE SET steward = true
         RETURNING role_id",
    )
    .bind(workspace_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.membership_security_roles (membership_id, role_id)
         SELECT m.membership_id, $2
         FROM public.memberships m
         WHERE m.user_id = $1 AND m.workspace_id = $3 AND m.is_active
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(role_id.0)
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.workspace_clearance_settings (workspace_id, enforcement, policy_epoch)
         VALUES ($1, true, 1)
         ON CONFLICT (workspace_id) DO UPDATE
         SET enforcement = true, policy_epoch = public.workspace_clearance_settings.policy_epoch + 1",
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn copy_labels(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<(), ApiError> {
    for table in [
        "chunks",
        "document_originals",
        "pdf_documents",
        "document_mm_assets",
        "document_pages",
        "tasks",
        "graph_contributions",
    ] {
        let sql = match table {
            "graph_contributions" => "UPDATE public.graph_contributions AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.source_document_id = d.id
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
                .to_string(),
            "tasks" => "UPDATE public.tasks AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.document_id = d.id::text
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
                .to_string(),
            _ => format!(
                "UPDATE public.{table} AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.document_id = d.id
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
            ),
        };
        sqlx::query(&sql)
            .bind(workspace_id)
            .execute(&mut **tx)
            .await
            .map_err(|error| ApiError::Internal(format!("{table}: {error}")))?;
    }
    Ok(())
}

/// Store an omitted upload label as unverified.
#[cfg(feature = "postgres")]
pub async fn stamp_unverified(
    pool: &sqlx::PgPool,
    document_id: Uuid,
    uploaded_by: Option<Uuid>,
) -> Result<(), ApiError> {
    let session = ClearanceSession::current();
    let enforced = session.as_ref().is_some_and(|session| session.enforced);
    if !enforced {
        return Ok(());
    }
    sqlx::query(
        "UPDATE public.documents
         SET classification_rank = $2, uploaded_by = COALESCE(uploaded_by, $3),
             compartment_ids = COALESCE(compartment_ids, '{}')
         WHERE id = $1 AND classification_rank IS NULL",
    )
    .bind(document_id)
    .bind(i16::from(RANK_UNVERIFIED))
    .bind(uploaded_by)
    .execute(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(())
}

/// What a relabel asks for: the document and its new classification and departments.
#[cfg(feature = "postgres")]
pub struct RelabelRequest<'a> {
    pub workspace_id: Uuid,
    pub document_id: Uuid,
    pub classification: &'a str,
    pub compartment_ids: &'a [Uuid],
}

#[cfg(feature = "postgres")]
pub async fn relabel(
    pool: &sqlx::PgPool,
    request: RelabelRequest<'_>,
    scope: &VisibilityScope,
    tenant_id: &str,
    actor: &str,
) -> Result<(), ApiError> {
    let RelabelRequest {
        workspace_id,
        document_id,
        classification,
        compartment_ids,
    } = request;
    let rank = rank_from_name(classification)
        .ok_or_else(|| ApiError::BadRequest("Unknown classification".into()))?;
    let current: Option<(Option<i16>, Vec<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT classification_rank, COALESCE(compartment_ids, '{}'), uploaded_by
         FROM public.documents WHERE id = $1 AND workspace_id = $2",
    )
    .bind(document_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let Some((old_rank, old_tags, uploaded_by)) = current else {
        return Err(not_found(&document_id.to_string()));
    };
    let before = ObjectLabel {
        rank: old_rank.and_then(|rank| u8::try_from(rank).ok()),
        compartments: old_tags.iter().copied().collect(),
        uploaded_by,
    };
    if !allows(&scope.subject, &before, ClearanceAction::Read).is_allow() {
        return Err(not_found(&document_id.to_string()));
    }
    let mut after = ObjectLabel::new(rank, compartment_ids.iter().copied());
    after.uploaded_by = uploaded_by;
    if !allows_relabel(&scope.subject, &before, &after).is_allow() {
        return Err(ApiError::forbidden_reason(
            "That label is outside this access profile",
        ));
    }
    let owned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.clearance_tags
         WHERE workspace_id = $1 AND kind = 'compartment' AND tag_id = ANY($2)",
    )
    .bind(workspace_id)
    .bind(compartment_ids)
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    if owned != compartment_ids.len() as i64 {
        return Err(ApiError::BadRequest("Unknown department".into()));
    }
    let downgrade = before.rank.is_some_and(|old| rank < old)
        || !before.compartments.is_subset(&after.compartments);
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query("DELETE FROM public.document_labels WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    if !compartment_ids.is_empty() {
        sqlx::query(
            "INSERT INTO public.document_labels (document_id, tag_id)
             SELECT $1, tag_id FROM unnest($2::uuid[]) AS tag_id",
        )
        .bind(document_id)
        .bind(compartment_ids)
        .execute(&mut *tx)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    }
    let tags = compartment_ids.to_vec();
    sqlx::query(
        "UPDATE public.documents
         SET classification_rank = $2, compartment_ids = $3
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(i16::from(rank))
    .bind(&tags)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    copy_one(&mut tx, document_id, i16::from(rank), &tags, uploaded_by).await?;
    sqlx::query(
        "UPDATE public.workspace_clearance_settings
         SET policy_epoch = policy_epoch + 1
         WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    if downgrade {
        sqlx::query(
            "INSERT INTO public.audit_logs
               (tenant_id, workspace_id, user_id, event_type, event_category, event_action,
                resource_type, resource_id, result, severity)
             VALUES ($1, $2, $3, 'Authorization', 'document_clearance', 'clearance.downgrade',
                     'document', $4, 'Success', 'Medium')",
        )
        .bind(tenant_id)
        .bind(workspace_id.to_string())
        .bind(actor)
        .bind(document_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    }
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn copy_one(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    document_id: Uuid,
    rank: i16,
    tags: &[Uuid],
    uploaded_by: Option<Uuid>,
) -> Result<(), ApiError> {
    for (sql, text_id) in [
        (
            "UPDATE public.chunks SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            false,
        ),
        (
            "UPDATE public.document_originals SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            false,
        ),
        (
            "UPDATE public.pdf_documents SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            false,
        ),
        (
            "UPDATE public.document_mm_assets SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            false,
        ),
        (
            "UPDATE public.document_pages SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            false,
        ),
        (
            "UPDATE public.graph_contributions SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE source_document_id = $1",
            false,
        ),
        (
            "UPDATE public.tasks SET classification_rank = $2, compartment_ids = $3, uploaded_by = $4 WHERE document_id = $1",
            true,
        ),
    ] {
        let query = sqlx::query(sql).bind(if text_id {
            // rebound below
            document_id
        } else {
            document_id
        });
        let result = if text_id {
            sqlx::query(sql)
                .bind(document_id.to_string())
                .bind(rank)
                .bind(tags)
                .bind(uploaded_by)
                .execute(&mut **tx)
                .await
        } else {
            query
                .bind(rank)
                .bind(tags)
                .bind(uploaded_by)
                .execute(&mut **tx)
                .await
        };
        result.map_err(|error| ApiError::Internal(error.to_string()))?;
    }
    Ok(())
}
