//! Operator text for `edgequake originals`. Secrets never appear.

#[derive(Debug, Clone)]
pub struct PlanView {
    pub database_url: String,
    pub store_url: String,
    pub mode: String,
    pub role: String,
    pub pdf_rows: u64,
    pub pdf_bytes: u64,
    pub pdf_verified: u64,
    pub original_rows: u64,
    pub original_bytes: u64,
    pub original_verified: u64,
    pub asset_rows: u64,
    pub asset_bytes: u64,
    pub asset_verified: u64,
    pub markdown_rows: u64,
    pub markdown_bytes: u64,
    pub markdown_verified: u64,
    pub pdf_relation_bytes: u64,
    pub original_relation_bytes: u64,
    pub asset_relation_bytes: u64,
    pub markdown_relation_bytes: u64,
    pub blocked: u64,
    pub next: String,
}

#[derive(Debug, Clone)]
pub struct ClearView {
    pub pdf: u64,
    pub original: u64,
    pub asset: u64,
    pub markdown: u64,
    pub bytes: u64,
    pub pdf_relation_bytes: u64,
    pub original_relation_bytes: u64,
    pub asset_relation_bytes: u64,
    pub markdown_relation_bytes: u64,
}

fn pending_rows(view: &PlanView) -> u64 {
    view.pdf_rows + view.original_rows + view.asset_rows + view.markdown_rows
}

pub fn suggest_next(view: &PlanView) -> &'static str {
    if pending_rows(view) + view.blocked == 0 {
        "none"
    } else if pending_rows(view) == 0 {
        "edgequake originals verify"
    } else {
        "edgequake originals migrate"
    }
}

pub fn render_plan(view: &PlanView) -> String {
    let space = view.pdf_relation_bytes
        + view.original_relation_bytes
        + view.asset_relation_bytes
        + view.markdown_relation_bytes;
    format!(
        "\
originals  database={db}  store={store}
mode       {mode}
role       {role}
pdf        {pdf_rows} rows   {pdf_bytes}   already verified {pdf_verified}
original   {orig_rows} rows   {orig_bytes}   already verified {orig_verified}
asset      {asset_rows} rows   {asset_bytes}   already verified {asset_verified}
markdown   {md_rows} rows   {md_bytes}   already verified {md_verified}
relations  pdf_document_blobs {pdf_rel} (pg_total_relation_size)   document_originals {orig_rel} (pg_total_relation_size)   document_mm_assets {asset_rel} (pg_total_relation_size)   pdf_documents {md_rel} (pg_total_relation_size)
space      VACUUM FULL needs about {space} free space after clear
note       a versioned bucket turns DeleteObject into a delete marker
note       a Postgres PITR without the bucket is not a full restore
blocked    {blocked}
next       {next}
",
        db = view.database_url,
        store = view.store_url,
        mode = view.mode,
        role = view.role,
        pdf_rows = view.pdf_rows,
        pdf_bytes = human_bytes(view.pdf_bytes),
        pdf_verified = view.pdf_verified,
        orig_rows = view.original_rows,
        orig_bytes = human_bytes(view.original_bytes),
        orig_verified = view.original_verified,
        asset_rows = view.asset_rows,
        asset_bytes = human_bytes(view.asset_bytes),
        asset_verified = view.asset_verified,
        md_rows = view.markdown_rows,
        md_bytes = human_bytes(view.markdown_bytes),
        md_verified = view.markdown_verified,
        pdf_rel = human_bytes(view.pdf_relation_bytes),
        orig_rel = human_bytes(view.original_relation_bytes),
        asset_rel = human_bytes(view.asset_relation_bytes),
        md_rel = human_bytes(view.markdown_relation_bytes),
        space = human_bytes(space),
        blocked = view.blocked,
        next = suggest_next(view),
    )
}

/// Counts by state. No disk estimate, because `status` does not plan a copy.
pub fn render_status(view: &PlanView) -> String {
    format!(
        "\
status     read-only
mode       {mode}
role       {role}
pdf        bytea {pdf_rows} ({pdf_bytes})  verified {pdf_verified}
original   bytea {orig_rows} ({orig_bytes})  verified {orig_verified}
asset      bytea {asset_rows} ({asset_bytes})  verified {asset_verified}
markdown   text {md_rows} ({md_bytes})  verified {md_verified}
blocked    {blocked} unverified refs
next       {next}
",
        mode = view.mode,
        role = view.role,
        pdf_rows = view.pdf_rows,
        pdf_bytes = human_bytes(view.pdf_bytes),
        pdf_verified = view.pdf_verified,
        orig_rows = view.original_rows,
        orig_bytes = human_bytes(view.original_bytes),
        orig_verified = view.original_verified,
        asset_rows = view.asset_rows,
        asset_bytes = human_bytes(view.asset_bytes),
        asset_verified = view.asset_verified,
        md_rows = view.markdown_rows,
        md_bytes = human_bytes(view.markdown_bytes),
        md_verified = view.markdown_verified,
        blocked = view.blocked,
        next = suggest_next(view),
    )
}

pub fn render_clear(view: &ClearView) -> String {
    let space = view.pdf_relation_bytes
        + view.original_relation_bytes
        + view.asset_relation_bytes
        + view.markdown_relation_bytes;
    format!(
        "\
cleared pdf {pdf}  original {original}  asset {asset}  markdown {markdown}  bytes {bytes}
disk    unchanged until VACUUM FULL or pg_repack
        tables: pdf_document_blobs, document_originals, document_mm_assets, pdf_documents
        free space required: about {space} (the rewrite is a second copy)
next    psql -c \"VACUUM (FULL, ANALYZE) pdf_document_blobs, document_originals, document_mm_assets, pdf_documents\"
",
        pdf = view.pdf,
        original = view.original,
        asset = view.asset,
        markdown = view.markdown,
        bytes = human_bytes(view.bytes),
        space = human_bytes(space),
    )
}

pub fn render_migrate(
    copied: u64,
    total: u64,
    verified: u64,
    skipped: u64,
    failed: u64,
    bytes: u64,
    elapsed: std::time::Duration,
) -> String {
    format!(
        "\
copy    {copied}/{total}  verified {verified}  skipped {skipped}  failed {failed}
bytes   {bytes}
time    {elapsed}
kept    BYTEA still in pdf_document_blobs, document_originals, and document_mm_assets; markdown_content still in pdf_documents
next    edgequake originals clear --yes
note    clear does not shrink the disk. After clear, run VACUUM FULL
        or pg_repack on pdf_document_blobs, document_originals, document_mm_assets, and pdf_documents.
        documents.content is not cleared.
",
        bytes = human_bytes(bytes),
        elapsed = format_elapsed(elapsed),
    )
}

pub fn redact_database_url(url: &str) -> String {
    // Drop userinfo. `postgres://user:secret@host/db` becomes `postgres://host/db`.
    if let Some(scheme) = url.find("://") {
        let rest = &url[scheme + 3..];
        if let Some(at) = rest.rfind('@') {
            return format!("{}{}", &url[..scheme + 3], &rest[at + 1..]);
        }
    }
    url.to_string()
}

pub fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let n = bytes as f64;
    if n >= KIB * KIB * KIB {
        format!("{:.1} GiB", n / (KIB * KIB * KIB))
    } else if n >= KIB * KIB {
        format!("{:.0} MiB", n / (KIB * KIB))
    } else if n >= KIB {
        format!("{:.0} KiB", n / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn format_elapsed(elapsed: std::time::Duration) -> String {
    let secs = elapsed.as_secs();
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PlanView {
        PlanView {
            database_url: "postgres://localhost/edgequake".into(),
            store_url: "s3://eq-originals/prod".into(),
            mode: "object".into(),
            role: "edgequake_originals_maint".into(),
            pdf_rows: 2,
            pdf_bytes: 10,
            pdf_verified: 0,
            original_rows: 1,
            original_bytes: 4,
            original_verified: 0,
            asset_rows: 4,
            asset_bytes: 8,
            asset_verified: 0,
            markdown_rows: 1,
            markdown_bytes: 12,
            markdown_verified: 0,
            pdf_relation_bytes: 100,
            original_relation_bytes: 40,
            asset_relation_bytes: 16,
            markdown_relation_bytes: 20,
            blocked: 0,
            next: "edgequake originals migrate".into(),
        }
    }

    #[test]
    fn plan_names_vacuum_full_relation_size_and_free_space() {
        let text = render_plan(&sample());
        assert!(text.contains("VACUUM FULL"));
        assert!(text.contains("pdf_document_blobs"));
        assert!(text.contains("document_originals"));
        assert!(text.contains("pg_total_relation_size"));
        assert!(text.contains("free space"));
        assert!(text.contains("delete marker"));
        assert!(text.contains("PITR"));
        assert!(text.contains("edgequake originals migrate"));
    }

    #[test]
    fn status_counts_states_and_skips_the_disk_estimate() {
        let text = render_status(&sample());
        assert!(text.contains("read-only"));
        assert!(text.contains("bytea 2"));
        assert!(text.contains("verified 0"));
        assert!(!text.contains("VACUUM FULL"));
        assert!(!text.contains("pg_total_relation_size"));
    }

    #[test]
    fn an_empty_plan_has_nothing_to_run_next() {
        let mut view = sample();
        view.pdf_rows = 0;
        view.original_rows = 0;
        view.asset_rows = 0;
        view.markdown_rows = 0;
        view.blocked = 0;
        assert_eq!(suggest_next(&view), "none");
        assert!(render_plan(&view).contains("next       none"));
    }

    #[test]
    fn clear_names_the_tables_and_vacuum_full() {
        let text = render_clear(&ClearView {
            pdf: 1,
            original: 1,
            asset: 1,
            markdown: 1,
            bytes: 14,
            pdf_relation_bytes: 100,
            original_relation_bytes: 40,
            asset_relation_bytes: 16,
            markdown_relation_bytes: 20,
        });
        assert!(text.contains("VACUUM FULL"));
        assert!(text.contains("pdf_document_blobs"));
        assert!(text.contains("document_originals"));
        assert!(text.contains("document_mm_assets"));
        assert!(text.contains("pdf_documents"));
        assert!(text.contains("free space"));
    }

    #[test]
    fn userinfo_is_stripped() {
        let redacted = redact_database_url("postgres://edgequake:s3cret@localhost/edgequake");
        assert!(!redacted.contains("s3cret"));
        assert!(redacted.contains("localhost/edgequake"));
    }
}
