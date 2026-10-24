-- SPEC-173 agent upload sessions. Bytes for a relay or chunk stay here until commit.
-- Staging object keys are not original_object_refs. There is no down migration.

SET search_path = public;

CREATE TABLE IF NOT EXISTS mcp_upload_sessions (
    upload_id text PRIMARY KEY,
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    user_id uuid,
    filename text NOT NULL,
    media_type text NOT NULL,
    byte_length bigint,
    sha256 text,
    staging_key text,
    staging_bytes bytea,
    transfer text NOT NULL,
    bind text NOT NULL,
    status text NOT NULL,
    received bigint NOT NULL DEFAULT 0,
    document_id text,
    task_id text,
    commit_status text,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT mcp_upload_sessions_transfer_chk CHECK (transfer IN ('presign', 'relay', 'chunk')),
    CONSTRAINT mcp_upload_sessions_bind_chk CHECK (bind IN ('capability', 'bearer')),
    CONSTRAINT mcp_upload_sessions_status_chk CHECK (status IN ('open', 'committed', 'aborted', 'expired'))
);

CREATE INDEX IF NOT EXISTS mcp_upload_sessions_reap
    ON mcp_upload_sessions (status, expires_at);

ALTER TABLE public.mcp_upload_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.mcp_upload_sessions FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_access_allow ON public.mcp_upload_sessions;
DROP POLICY IF EXISTS tenant_access_guard ON public.mcp_upload_sessions;

CREATE POLICY tenant_access_allow ON public.mcp_upload_sessions
    FOR ALL TO edgequake_tenant_access
    USING (true) WITH CHECK (true);

CREATE POLICY tenant_access_guard ON public.mcp_upload_sessions
    AS RESTRICTIVE FOR ALL TO PUBLIC
    USING (
        (
            tenant_id = public.current_tenant_id()
            AND (
                public.current_workspace_id() IS NULL
                OR workspace_id = public.current_workspace_id()
            )
        )
        OR current_setting('edgequake.mcp_upload_reaper', true) = '1'
    )
    WITH CHECK (
        (
            tenant_id = public.current_tenant_id()
            AND (
                public.current_workspace_id() IS NULL
                OR workspace_id = public.current_workspace_id()
            )
        )
        OR current_setting('edgequake.mcp_upload_reaper', true) = '1'
    );

GRANT SELECT, INSERT, UPDATE, DELETE ON mcp_upload_sessions TO edgequake_tenant_access;
