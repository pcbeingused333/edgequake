-- SPEC-176: tenant secret backends. Expand-only. Existing provider key columns stay.
-- The database backend is the default. Infisical is optional and stores no value here.
-- A missing row means the tenant uses the database backend.

SET search_path = public;

CREATE TABLE IF NOT EXISTS public.secret_backend_configs (
    tenant_id UUID PRIMARY KEY,
    kind TEXT NOT NULL DEFAULT 'database',
    infisical_base_url TEXT,
    infisical_client_id TEXT,
    infisical_project_id TEXT,
    infisical_environment TEXT,
    infisical_secret_path TEXT NOT NULL DEFAULT '/',
    allow_private_network BOOLEAN NOT NULL DEFAULT FALSE,
    client_secret_ciphertext BYTEA,
    client_secret_nonce BYTEA,
    client_secret_key_id TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT secret_backend_configs_kind_chk CHECK (kind IN ('database', 'infisical'))
);

CREATE TABLE IF NOT EXISTS public.managed_secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    key_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS managed_secrets_tenant_idx
    ON public.managed_secrets (tenant_id);

ALTER TABLE public.provider_connections
    ADD COLUMN IF NOT EXISTS secret_ref JSONB;

-- Nil tenant_id (00000000-0000-0000-0000-000000000000) is the deployment bucket
-- used when a request has no tenant. Real tenants use their own id.
ALTER TABLE public.secret_backend_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.secret_backend_configs FORCE ROW LEVEL SECURITY;
ALTER TABLE public.managed_secrets ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.managed_secrets FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_access_allow ON public.secret_backend_configs;
DROP POLICY IF EXISTS tenant_access_guard ON public.secret_backend_configs;
DROP POLICY IF EXISTS tenant_access_allow ON public.managed_secrets;
DROP POLICY IF EXISTS tenant_access_guard ON public.managed_secrets;

CREATE POLICY tenant_access_allow ON public.secret_backend_configs
    FOR ALL TO edgequake_tenant_access
    USING (true) WITH CHECK (true);
CREATE POLICY tenant_access_guard ON public.secret_backend_configs
    AS RESTRICTIVE FOR ALL TO PUBLIC
    USING (tenant_id = public.current_tenant_id())
    WITH CHECK (tenant_id = public.current_tenant_id());

CREATE POLICY tenant_access_allow ON public.managed_secrets
    FOR ALL TO edgequake_tenant_access
    USING (true) WITH CHECK (true);
CREATE POLICY tenant_access_guard ON public.managed_secrets
    AS RESTRICTIVE FOR ALL TO PUBLIC
    USING (tenant_id = public.current_tenant_id())
    WITH CHECK (tenant_id = public.current_tenant_id());

GRANT SELECT, INSERT, UPDATE, DELETE ON public.secret_backend_configs TO edgequake_tenant_access;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.managed_secrets TO edgequake_tenant_access;
