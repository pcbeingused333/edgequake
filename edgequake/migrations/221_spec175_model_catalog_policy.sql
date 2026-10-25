-- SPEC-175 W2: model catalog policy (tenant ceiling + workspace narrowing).
--
-- `workspace_id IS NULL` rows are the tenant ceiling. `workspace_id` set rows
-- narrow one workspace. An empty table means unconstrained (no backfill).
-- Two partial unique indexes because NULL workspace_id values are distinct in a
-- plain UNIQUE constraint. Expand-only: no existing table is altered in place.

CREATE TABLE IF NOT EXISTS model_catalog_policy (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL REFERENCES tenants (tenant_id) ON DELETE CASCADE,
    workspace_id  uuid NULL REFERENCES workspaces (workspace_id) ON DELETE CASCADE,
    provider      text NOT NULL,
    model         text NOT NULL,
    usage         text NOT NULL,
    enabled       boolean NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT model_catalog_policy_usage_chk CHECK (
        usage IN (
            'extract', 'query', 'summary', 'vlm', 'keyword',
            'embedding', 'decision', 'rerank'
        )
    ),
    CONSTRAINT model_catalog_policy_provider_chk CHECK (
        char_length(provider) BETWEEN 1 AND 64
    ),
    CONSTRAINT model_catalog_policy_model_chk CHECK (
        char_length(model) BETWEEN 1 AND 256
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS model_catalog_policy_ceiling_uq
    ON model_catalog_policy (tenant_id, provider, model, usage)
    WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS model_catalog_policy_workspace_uq
    ON model_catalog_policy (tenant_id, workspace_id, provider, model, usage)
    WHERE workspace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS model_catalog_policy_tenant_idx
    ON model_catalog_policy (tenant_id, usage)
    WHERE enabled;

-- The composite FK below needs a unique (tenant_id, workspace_id) pair on
-- workspaces. workspace_id is already the primary key, so this is a cheap
-- index that stops a narrowing row from naming another tenant's workspace.
CREATE UNIQUE INDEX IF NOT EXISTS workspaces_tenant_id_workspace_id_uq
    ON workspaces (tenant_id, workspace_id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'model_catalog_policy_workspace_tenant_fk'
    ) THEN
        ALTER TABLE model_catalog_policy
            ADD CONSTRAINT model_catalog_policy_workspace_tenant_fk
            FOREIGN KEY (tenant_id, workspace_id)
            REFERENCES workspaces (tenant_id, workspace_id);
    END IF;
END
$$;
