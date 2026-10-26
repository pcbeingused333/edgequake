-- SPEC-179: refresh cache, offer revision, fleet slug uniqueness.
-- Expand-only. Choice itself stays in the tables SPEC-175 and SPEC-177 own.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS provider_offer_revision integer NOT NULL DEFAULT 1;

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS provider_offer_revision integer NOT NULL DEFAULT 1;

CREATE UNIQUE INDEX IF NOT EXISTS provider_connections_fleet_slug_uq
    ON provider_connections (slug)
    WHERE tenant_id IS NULL;

CREATE INDEX IF NOT EXISTS provider_offers_slug_idx
    ON provider_offers (provider_slug);

CREATE INDEX IF NOT EXISTS model_catalog_policy_scope_usage_idx
    ON model_catalog_policy (tenant_id, workspace_id, usage);

CREATE TABLE IF NOT EXISTS provider_model_cache (
    provider_slug       text NOT NULL,
    model               text NOT NULL,
    supports_chat       boolean NOT NULL DEFAULT false,
    supports_vision     boolean NOT NULL DEFAULT false,
    supports_embedding  boolean NOT NULL DEFAULT false,
    embedding_dimension integer NULL,
    context_length      integer NULL,
    source              text NOT NULL DEFAULT 'refresh',
    first_seen_at       timestamptz NOT NULL DEFAULT now(),
    last_seen_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT provider_model_cache_pk PRIMARY KEY (provider_slug, model),
    CONSTRAINT provider_model_cache_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    ),
    CONSTRAINT provider_model_cache_model_chk CHECK (
        char_length(model) BETWEEN 1 AND 256
    )
);

CREATE TABLE IF NOT EXISTS provider_refresh_state (
    provider_slug     text PRIMARY KEY,
    last_refreshed_at timestamptz NOT NULL DEFAULT now(),
    status            text NOT NULL,
    error             text NULL,
    added_count       integer NOT NULL DEFAULT 0,
    removed_count     integer NOT NULL DEFAULT 0,
    CONSTRAINT provider_refresh_state_status_chk CHECK (status IN ('ok', 'error')),
    CONSTRAINT provider_refresh_state_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    )
);

CREATE TABLE IF NOT EXISTS provider_config_audit (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id    uuid NULL,
    action      text NOT NULL,
    scope       text NOT NULL,
    subject     text NOT NULL,
    before_hash text NULL,
    after_hash  text NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS provider_config_audit_created_idx
    ON provider_config_audit (created_at DESC);
