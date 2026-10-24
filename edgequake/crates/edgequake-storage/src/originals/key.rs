//! Object keys are ids. Filenames stay in Postgres (LAW-172-8).

use uuid::Uuid;

/// Which parent the key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnerKind {
    Pdf,
    Original,
    Asset,
    Markdown,
    /// In-flight agent upload. Not an original, and not a migrate target.
    Upload,
}

impl OwnerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Original => "original",
            Self::Asset => "asset",
            Self::Markdown => "markdown",
            Self::Upload => "upload",
        }
    }
}

fn kind_prefix(prefix: &str, kind: OwnerKind, tenant: Uuid, workspace: Uuid) -> String {
    let body = format!(
        "t/{}/w/{}/{}/",
        tenant.as_hyphenated(),
        workspace.as_hyphenated(),
        kind.as_str()
    );
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        body
    } else {
        format!("{prefix}/{body}")
    }
}

/// `{prefix}/t/{tenant}/w/{workspace}/pdf|original|asset|markdown/{id}`.
///
/// UUIDs are hyphenated lowercase. `filename` is not a parameter on purpose.
pub fn object_key(
    prefix: &str,
    kind: OwnerKind,
    tenant: Uuid,
    workspace: Uuid,
    owner: Uuid,
) -> String {
    format!(
        "{}{}",
        kind_prefix(prefix, kind, tenant, workspace),
        owner.as_hyphenated()
    )
}

/// The object key to read or delete, or `None` when the pointer belongs to
/// another tenant or workspace. A stored key is not authorization by itself.
///
/// The last segment may be any one uuid. A replace writes a fresh uuid and
/// keeps the same kind prefix. The row id is not part of this check.
pub fn scoped_object_key(
    prefix: &str,
    kind: OwnerKind,
    tenant: Uuid,
    workspace: Uuid,
    pointer_tenant: Uuid,
    pointer_workspace: Uuid,
    pointer_key: &str,
) -> Option<String> {
    if pointer_tenant != tenant || pointer_workspace != workspace {
        return None;
    }
    let root = kind_prefix(prefix, kind, tenant, workspace);
    let rest = pointer_key.strip_prefix(&root)?;
    if rest.contains('/') || Uuid::parse_str(rest).is_err() {
        return None;
    }
    Some(pointer_key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_never_enters_the_key() {
        let tenant = Uuid::nil();
        let workspace = Uuid::from_u128(1);
        let owner = Uuid::from_u128(2);
        let filename = "../café.pdf";
        let key = object_key("dev", OwnerKind::Pdf, tenant, workspace, owner);
        assert!(!key.contains(filename));
        assert!(!key.contains("café"));
        assert!(!key.contains(".."));
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(
            parts,
            [
                "dev",
                "t",
                &tenant.to_string(),
                "w",
                &workspace.to_string(),
                "pdf",
                &owner.to_string()
            ]
        );
        assert!(parts.iter().all(|p| !p.contains('/')));
    }

    #[test]
    fn another_tenant_or_workspace_is_not_the_object_key() {
        let tenant = Uuid::from_u128(1);
        let other = Uuid::from_u128(9);
        let workspace = Uuid::from_u128(2);
        let owner = Uuid::from_u128(3);
        let key = object_key("dev", OwnerKind::Pdf, tenant, workspace, owner);
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Pdf,
            tenant,
            workspace,
            tenant,
            workspace,
            &key
        )
        .is_some());
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Pdf,
            tenant,
            workspace,
            other,
            workspace,
            &key
        )
        .is_none());
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Original,
            tenant,
            workspace,
            tenant,
            other,
            &key
        )
        .is_none());
        let foreign = object_key("dev", OwnerKind::Pdf, other, workspace, owner);
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Pdf,
            tenant,
            workspace,
            tenant,
            workspace,
            &foreign
        )
        .is_none());
    }

    #[test]
    fn asset_and_markdown_keys_omit_path_and_filename() {
        let tenant = Uuid::from_u128(1);
        let workspace = Uuid::from_u128(2);
        let owner = Uuid::from_u128(3);
        let asset = object_key("dev", OwnerKind::Asset, tenant, workspace, owner);
        let markdown = object_key("dev", OwnerKind::Markdown, tenant, workspace, owner);
        for key in [&asset, &markdown] {
            assert!(!key.contains("page-0001.png"));
            assert!(!key.contains("assets/"));
            assert!(!key.contains("café"));
            assert!(!key.contains(".md"));
            assert!(!key.contains(".."));
        }
        assert!(asset.contains("/asset/"));
        assert!(markdown.contains("/markdown/"));
        let other = Uuid::from_u128(9);
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            other,
            workspace,
            &asset
        )
        .is_none());
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Markdown,
            tenant,
            workspace,
            tenant,
            other,
            &markdown
        )
        .is_none());
    }

    #[test]
    fn a_fresh_uuid_under_the_kind_prefix_is_readable() {
        let tenant = Uuid::from_u128(1);
        let workspace = Uuid::from_u128(2);
        let owner = Uuid::from_u128(3);
        let fresh = object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            Uuid::from_u128(99),
        );
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            tenant,
            workspace,
            &fresh
        )
        .is_some());
        let outside = format!("other/{fresh}");
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            tenant,
            workspace,
            &outside
        )
        .is_none());
        let extra = format!("{fresh}/more");
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            tenant,
            workspace,
            &extra
        )
        .is_none());
        let kind = object_key("dev", OwnerKind::Pdf, tenant, workspace, owner);
        assert!(scoped_object_key(
            "dev",
            OwnerKind::Asset,
            tenant,
            workspace,
            tenant,
            workspace,
            &kind
        )
        .is_none());
    }
}
