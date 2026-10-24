//! MCP profile: query lockdown vs control (write tools advertised).

/// Which tool surface the gateway advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProfile {
    /// Read-only catalog + retrieval + graph (lockdown).
    Query,
    /// Control surface: ingest, upload, delete, download, assets, graph image.
    Control,
    /// Alias of Control (SPEC-152 env name).
    Memory,
}

impl McpProfile {
    /// Write tools are listed and callable.
    pub fn advertises_writes(self) -> bool {
        matches!(self, Self::Control | Self::Memory)
    }
}

/// Resolve profile from `EDGEQUAKE_MCP_PROFILE`. Default: control.
pub fn mcp_profile() -> McpProfile {
    match std::env::var("EDGEQUAKE_MCP_PROFILE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "query" => McpProfile::Query,
        "memory" => McpProfile::Memory,
        _ => McpProfile::Control,
    }
}

pub const QUERY_INSTRUCTIONS: &str = r#"EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding the corpus is small.
6. Ignore Artifact/DRAWING entities unless the user asks about a figure.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. This connector is query-only; ingest and delete are unavailable."#;

pub const CONTROL_INSTRUCTIONS: &str = r#"EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding the corpus is small.
6. Ignore Artifact/DRAWING unless the user asks about a figure; use eq_asset_get for pixels.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. Ingest and upload are async. eq_ingest / eq_upload_commit return status pending and a task_id (track_id). Poll eq_task_get until status is indexed (success) or failed. A file such as a PDF uses eq_upload_begin, then an HTTP PUT of the raw bytes to the returned url with the returned headers, then eq_upload_commit. Do not place file bytes or the url in the chat. eq_document_get returns the pending shell from staging metadata; ready is false until indexed. Markdown eq_document_download and eq://…/text return eq/not_ready until indexed. Do not claim the document is searchable before indexed. Deletes require confirm: true; they return accepted plus task_id with deleted false until the deletion task is indexed. Download with eq_document_download. Graph picture: eq_graph_image centered on an entity_id."#;

pub const MEMORY_INSTRUCTIONS: &str = CONTROL_INSTRUCTIONS;

pub fn instructions_for_profile(profile: McpProfile) -> &'static str {
    match profile {
        McpProfile::Query => QUERY_INSTRUCTIONS,
        McpProfile::Control | McpProfile::Memory => CONTROL_INSTRUCTIONS,
    }
}
