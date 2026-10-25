-- SPEC-177: provider visibility. Expand-only.
-- provider_offer_constrained false means every provider the parent offers.
-- Rows store the enabled slugs once the flag is true. An empty row set with
-- the flag true means the scope offers nothing.

CREATE TABLE IF NOT EXISTS deployment_provider_overrides (
    provider_slug  text PRIMARY KEY,
    withdrawn      boolean NOT NULL,
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT deployment_provider_overrides_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    )
);

CREATE TABLE IF NOT EXISTS provider_offers (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL REFERENCES tenants (tenant_id) ON DELETE CASCADE,
    workspace_id  uuid NULL REFERENCES workspaces (workspace_id) ON DELETE CASCADE,
    provider_slug text NOT NULL,
    enabled       boolean NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT provider_offers_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS provider_offers_tenant_uq
    ON provider_offers (tenant_id, provider_slug)
    WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS provider_offers_workspace_uq
    ON provider_offers (tenant_id, workspace_id, provider_slug)
    WHERE workspace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS provider_offers_tenant_idx
    ON provider_offers (tenant_id)
    WHERE workspace_id IS NULL;

ALTER TABLE provider_offers
    ADD CONSTRAINT provider_offers_workspace_tenant_fk
    FOREIGN KEY (tenant_id, workspace_id)
    REFERENCES workspaces (tenant_id, workspace_id);

CREATE TABLE IF NOT EXISTS connection_models (
    connection_id  uuid NOT NULL REFERENCES provider_connections (id) ON DELETE CASCADE,
    model          text NOT NULL,
    supports_chat  boolean NOT NULL DEFAULT false,
    supports_vision boolean NOT NULL DEFAULT false,
    supports_embedding boolean NOT NULL DEFAULT false,
    embedding_dimension integer NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT connection_models_model_chk CHECK (
        char_length(model) BETWEEN 1 AND 256
    ),
    PRIMARY KEY (connection_id, model)
);

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS provider_offer_constrained boolean NOT NULL DEFAULT false;

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS provider_offer_constrained boolean NOT NULL DEFAULT false;
