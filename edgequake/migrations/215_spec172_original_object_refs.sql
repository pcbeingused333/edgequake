-- ============================================================================
-- Migration 215: SPEC-172 original object pointers
-- ============================================================================
-- Original bytes may live in pdf_document_blobs.pdf_data / document_originals.original_data
-- (byte_location = database) or in a private bucket (byte_location = object).
-- The pointer is original_object_refs. A CHECK cannot subquery that table
-- (pg_restore would hide it). The same-row checks below are the whole rule.
--
-- There is no down migration. The migrate engine is forward-only (SPEC-150).
-- To make the BYTEA columns NOT NULL again, first confirm original_object_refs
-- is empty and every byte_location is database, then ALTER the columns.
--
-- edgequake_originals_maint is BYPASSRLS and is not granted to the API role.
-- A superuser running `edgequake originals` uses SET ROLE. Direct login is:
--   ALTER ROLE edgequake_originals_maint LOGIN PASSWORD '...';

SET search_path = public;

ALTER TABLE pdf_document_blobs
    ADD COLUMN IF NOT EXISTS byte_location text NOT NULL DEFAULT 'database';
ALTER TABLE pdf_document_blobs
    ALTER COLUMN pdf_data DROP NOT NULL;
ALTER TABLE pdf_document_blobs
    DROP CONSTRAINT IF EXISTS pdf_document_blobs_byte_location_chk;
ALTER TABLE pdf_document_blobs
    ADD CONSTRAINT pdf_document_blobs_byte_location_chk
    CHECK (
        byte_location IN ('database', 'object')
        AND (
            (byte_location = 'database' AND pdf_data IS NOT NULL)
            OR (byte_location = 'object' AND pdf_data IS NULL)
        )
    );

ALTER TABLE document_originals
    ADD COLUMN IF NOT EXISTS byte_location text NOT NULL DEFAULT 'database';
ALTER TABLE document_originals
    ALTER COLUMN original_data DROP NOT NULL;
ALTER TABLE document_originals
    DROP CONSTRAINT IF EXISTS document_originals_byte_location_chk;
ALTER TABLE document_originals
    ADD CONSTRAINT document_originals_byte_location_chk
    CHECK (
        byte_location IN ('database', 'object')
        AND (
            (byte_location = 'database' AND original_data IS NOT NULL)
            OR (byte_location = 'object' AND original_data IS NULL)
        )
    );

CREATE TABLE IF NOT EXISTS original_object_refs (
    pdf_id       uuid NULL REFERENCES pdf_documents(pdf_id) ON DELETE CASCADE,
    document_id  uuid NULL REFERENCES documents(id) ON DELETE CASCADE,
    tenant_id    uuid NOT NULL,
    workspace_id uuid NOT NULL,
    object_key   text NOT NULL,
    sha256       char(64) NOT NULL,
    byte_length  bigint NOT NULL,
    content_type text NOT NULL,
    etag         text NULL,
    verified_at  timestamptz NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT original_object_refs_one_owner CHECK (num_nonnulls(pdf_id, document_id) = 1),
    CONSTRAINT original_object_refs_sha256_hex CHECK (sha256 ~ '^[a-f0-9]{64}$'),
    CONSTRAINT original_object_refs_object_key_key UNIQUE (object_key)
);

CREATE UNIQUE INDEX IF NOT EXISTS original_object_refs_pdf_id_key
    ON original_object_refs (pdf_id) WHERE pdf_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS original_object_refs_document_id_key
    ON original_object_refs (document_id) WHERE document_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS original_object_inflight (
    object_key text PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS original_object_deletes (
    object_key   text PRIMARY KEY,
    enqueued_at  timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION public.original_object_refs_enqueue_delete()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    INSERT INTO original_object_deletes (object_key, enqueued_at)
    VALUES (OLD.object_key, now())
    ON CONFLICT (object_key) DO NOTHING;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS original_object_refs_before_delete ON original_object_refs;
CREATE TRIGGER original_object_refs_before_delete
    BEFORE DELETE ON original_object_refs
    FOR EACH ROW
    EXECUTE FUNCTION public.original_object_refs_enqueue_delete();

-- Relation size for `edgequake originals plan`. The maintenance role is not
-- the table owner, and pg_total_relation_size is not granted to it on PG15+.
CREATE OR REPLACE FUNCTION public.original_relation_bytes(rel regclass)
RETURNS bigint
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT pg_total_relation_size(rel)
$$;

REVOKE ALL ON FUNCTION public.original_relation_bytes(regclass) FROM PUBLIC;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_originals_maint') THEN
        CREATE ROLE edgequake_originals_maint NOLOGIN BYPASSRLS;
    END IF;
END $$;

GRANT SELECT, UPDATE ON pdf_document_blobs, document_originals TO edgequake_originals_maint;
GRANT SELECT ON pdf_documents, documents, workspaces TO edgequake_originals_maint;
GRANT SELECT, INSERT, UPDATE, DELETE ON original_object_refs, original_object_inflight, original_object_deletes TO edgequake_originals_maint;
GRANT EXECUTE ON FUNCTION public.original_relation_bytes(regclass) TO edgequake_originals_maint;

-- Do not GRANT edgequake_originals_maint to the API user.

-- Workspace-scoped RLS, same shape as migration 167 for document_originals.
DO $$
DECLARE predicate text;
BEGIN
    IF to_regclass('public.original_object_refs') IS NULL THEN
        RETURN;
    END IF;
    predicate := 'workspace_id = public.current_workspace_id() AND EXISTS (SELECT 1 FROM public.workspaces w WHERE w.workspace_id = public.current_workspace_id() AND w.tenant_id = public.current_tenant_id())';
    EXECUTE 'ALTER TABLE public.original_object_refs ENABLE ROW LEVEL SECURITY';
    EXECUTE 'ALTER TABLE public.original_object_refs FORCE ROW LEVEL SECURITY';
    EXECUTE 'DROP POLICY IF EXISTS tenant_access_allow ON public.original_object_refs';
    EXECUTE 'DROP POLICY IF EXISTS tenant_access_guard ON public.original_object_refs';
    EXECUTE 'CREATE POLICY tenant_access_allow ON public.original_object_refs FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)';
    EXECUTE format(
        'CREATE POLICY tenant_access_guard ON public.original_object_refs AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)',
        predicate, predicate
    );
END $$;

GRANT SELECT, INSERT, UPDATE, DELETE ON original_object_refs TO edgequake_tenant_access;
