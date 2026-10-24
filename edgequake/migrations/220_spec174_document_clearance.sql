-- SPEC-174 document clearance. Additive. Does not turn enforcement on and
-- does not backfill ranks. Existing rows stay readable until an owner enables
-- clearance, because clearance_covers is true while the workspace flag is off.

SET search_path = public;

CREATE TABLE IF NOT EXISTS workspace_clearance_settings (
    workspace_id uuid PRIMARY KEY REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    enforcement boolean NOT NULL DEFAULT false,
    policy_epoch bigint NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS clearance_tags (
    tag_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id uuid NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('classification', 'compartment')),
    key text NOT NULL,
    value text NOT NULL,
    rank smallint,
    UNIQUE (workspace_id, kind, key, value)
);

CREATE TABLE IF NOT EXISTS security_roles (
    role_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id uuid NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    name text NOT NULL,
    steward boolean NOT NULL DEFAULT false,
    UNIQUE (workspace_id, name)
);

CREATE TABLE IF NOT EXISTS security_role_rules (
    rule_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id uuid NOT NULL REFERENCES security_roles(role_id) ON DELETE CASCADE,
    position integer NOT NULL,
    effect text NOT NULL CHECK (effect IN ('allow', 'deny')),
    match_kind text NOT NULL CHECK (match_kind IN (
        'classification_at_most', 'any_of', 'all_of', 'untagged'
    )),
    rank smallint,
    tag_ids uuid[] NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS membership_security_roles (
    membership_id uuid NOT NULL REFERENCES memberships(membership_id) ON DELETE CASCADE,
    role_id uuid NOT NULL REFERENCES security_roles(role_id) ON DELETE CASCADE,
    PRIMARY KEY (membership_id, role_id)
);

CREATE TABLE IF NOT EXISTS document_labels (
    document_id uuid NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tag_id uuid NOT NULL REFERENCES clearance_tags(tag_id) ON DELETE RESTRICT,
    PRIMARY KEY (document_id, tag_id)
);

ALTER TABLE documents ADD COLUMN IF NOT EXISTS uploaded_by uuid;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS classification_rank smallint;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS compartment_ids uuid[];

DO $$
DECLARE
    tbl text;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'chunks', 'document_originals', 'pdf_documents', 'document_mm_assets',
        'document_pages', 'tasks', 'graph_contributions'
    ] LOOP
        IF to_regclass(format('public.%I', tbl)) IS NULL THEN
            CONTINUE;
        END IF;
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS classification_rank smallint', tbl);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS compartment_ids uuid[]', tbl);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS uploaded_by uuid', tbl);
    END LOOP;
END $$;

ALTER TABLE graph_contributions ADD COLUMN IF NOT EXISTS kind text;
ALTER TABLE graph_contributions ADD COLUMN IF NOT EXISTS fragment text;
ALTER TABLE graph_contributions ADD COLUMN IF NOT EXISTS entity_name text;
ALTER TABLE graph_contributions ADD COLUMN IF NOT EXISTS chunk_key text;

UPDATE graph_contributions
SET kind = CASE payload->>'kind' WHEN 'node' THEN 'entity' WHEN 'edge' THEN 'edge' ELSE payload->>'kind' END
WHERE kind IS NULL AND payload IS NOT NULL;

UPDATE graph_contributions
SET entity_name = COALESCE(payload->>'node_id', payload->>'source')
WHERE entity_name IS NULL AND payload IS NOT NULL;

UPDATE graph_contributions
SET fragment = payload->'properties'->>'description'
WHERE fragment IS NULL AND payload IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_clearance_rank
    ON documents (workspace_id, classification_rank);
CREATE INDEX IF NOT EXISTS idx_chunks_clearance_rank
    ON chunks (workspace_id, classification_rank);
CREATE INDEX IF NOT EXISTS idx_documents_compartments
    ON documents USING gin (compartment_ids);
CREATE INDEX IF NOT EXISTS idx_chunks_compartments
    ON chunks USING gin (compartment_ids);
CREATE INDEX IF NOT EXISTS idx_graph_contrib_entity_name
    ON graph_contributions (workspace_id, entity_name);
CREATE INDEX IF NOT EXISTS idx_graph_contrib_source_doc
    ON graph_contributions (workspace_id, source_document_id);

CREATE OR REPLACE FUNCTION public.current_clearance_rank()
RETURNS smallint
LANGUAGE sql
STABLE
LEAKPROOF
AS $$
    SELECT CASE
        WHEN COALESCE(current_setting('app.clearance_rank', true), '') = '' THEN NULL
        ELSE current_setting('app.clearance_rank', true)::smallint
    END
$$;

CREATE OR REPLACE FUNCTION public.current_clearance_compartments()
RETURNS uuid[]
LANGUAGE sql
STABLE
LEAKPROOF
AS $$
    SELECT CASE
        WHEN COALESCE(current_setting('app.clearance_compartments', true), '') = '' THEN '{}'::uuid[]
        ELSE current_setting('app.clearance_compartments', true)::uuid[]
    END
$$;

-- True when this workspace is not enforcing, when break-glass is set, or when
-- the compiled rank and compartments cover the row. An unset rank while
-- enforcement is on is false. Unset workspace context (workers that did not
-- bind a tenant) stays true so unlabeled corpora keep today's reads.
CREATE OR REPLACE FUNCTION public.clearance_covers(doc_rank smallint, doc_compartments uuid[])
RETURNS boolean
LANGUAGE sql
STABLE
LEAKPROOF
AS $$
    SELECT
        (SELECT current_setting('app.clearance_break_glass', true)) = 'on'
        OR (
            (SELECT current_setting('app.clearance_enforced', true)) IS DISTINCT FROM 'on'
            AND COALESCE(
                (SELECT s.enforcement
                 FROM public.workspace_clearance_settings s
                 WHERE s.workspace_id = (SELECT public.current_workspace_id())),
                false
            ) = false
        )
        OR (
            doc_rank IS NOT NULL
            AND (SELECT public.current_clearance_rank()) IS NOT NULL
            AND doc_rank <= (SELECT public.current_clearance_rank())
            AND (
                (
                    COALESCE(cardinality(doc_compartments), 0) = 0
                    AND (SELECT current_setting('app.clearance_allow_empty', true)) IS DISTINCT FROM 'off'
                )
                OR (
                    COALESCE(cardinality(doc_compartments), 0) > 0
                    AND doc_compartments <@ (SELECT public.current_clearance_compartments())
                )
            )
        )
$$;

-- Not LEAKPROOF: the uploader clause calls current_user_id(), which is not leakproof.
-- Policies inline the same predicate and wrap the leakproof rank check.
CREATE OR REPLACE FUNCTION public.clearance_row_visible(
    doc_rank smallint,
    doc_compartments uuid[],
    doc_uploaded_by uuid
) RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT public.clearance_covers(doc_rank, doc_compartments)
        OR (
            doc_uploaded_by IS NOT NULL
            AND doc_uploaded_by = (SELECT public.current_user_id())
            AND doc_rank = 5
        )
$$;

CREATE OR REPLACE FUNCTION public.set_clearance_context(
    p_enforced text,
    p_rank text,
    p_compartments text,
    p_allow_empty text,
    p_steward text,
    p_break_glass text
) RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM set_config('app.clearance_enforced', COALESCE(p_enforced, 'off'), true);
    PERFORM set_config('app.clearance_rank', COALESCE(p_rank, ''), true);
    PERFORM set_config('app.clearance_compartments', COALESCE(p_compartments, ''), true);
    PERFORM set_config('app.clearance_allow_empty', COALESCE(p_allow_empty, 'on'), true);
    PERFORM set_config('app.clearance_steward', COALESCE(p_steward, 'off'), true);
    PERFORM set_config('app.clearance_break_glass', COALESCE(p_break_glass, 'off'), true);
END
$$;

-- Copy the document label onto a contribution at insert. Relabel updates the
-- columns in the same transaction as the document. Enforcement off leaves
-- the copied rank NULL, and clearance_covers still allows the row.
CREATE OR REPLACE FUNCTION public.graph_contribution_copy_label()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.kind IS NULL AND NEW.payload IS NOT NULL THEN
        NEW.kind := CASE NEW.payload->>'kind'
            WHEN 'node' THEN 'entity'
            WHEN 'edge' THEN 'edge'
            ELSE NEW.payload->>'kind'
        END;
        NEW.entity_name := COALESCE(NEW.payload->>'node_id', NEW.payload->>'source');
        NEW.fragment := NEW.payload->'properties'->>'description';
        NEW.chunk_key := NEW.payload->'properties'->'source_chunk_ids'->>0;
    END IF;
    IF NEW.classification_rank IS NULL THEN
        SELECT d.classification_rank, d.compartment_ids, d.uploaded_by
          INTO NEW.classification_rank, NEW.compartment_ids, NEW.uploaded_by
        FROM public.documents d
        WHERE d.id = NEW.source_document_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS graph_contribution_copy_label ON graph_contributions;
CREATE TRIGGER graph_contribution_copy_label
    BEFORE INSERT ON graph_contributions
    FOR EACH ROW
    EXECUTE FUNCTION public.graph_contribution_copy_label();

DO $$
DECLARE
    tbl text;
    predicate text;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'documents', 'chunks', 'document_originals', 'pdf_documents',
        'document_mm_assets', 'document_pages', 'tasks', 'graph_contributions'
    ] LOOP
        IF to_regclass(format('public.%I', tbl)) IS NULL THEN
            CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM pg_attribute
            WHERE attrelid = to_regclass(format('public.%I', tbl))
              AND attname = 'classification_rank'
              AND NOT attisdropped
        ) THEN
            CONTINUE;
        END IF;
        predicate := '(SELECT public.clearance_covers(classification_rank, compartment_ids)) OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5)';
        IF tbl = 'tasks' THEN
            predicate := format('document_id IS NULL OR (%s)', predicate);
        END IF;
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', tbl);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', tbl);
        EXECUTE format('DROP POLICY IF EXISTS clearance_guard ON public.%I', tbl);
        EXECUTE format(
            'CREATE POLICY clearance_guard ON public.%I AS RESTRICTIVE FOR ALL TO edgequake_tenant_access USING ((SELECT %s)) WITH CHECK ((SELECT %s))',
            tbl, predicate, predicate
        );
    END LOOP;
END $$;

GRANT SELECT, INSERT, UPDATE, DELETE ON
    workspace_clearance_settings, clearance_tags, security_roles,
    security_role_rules, membership_security_roles, document_labels
    TO edgequake_tenant_access;
