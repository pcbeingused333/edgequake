-- ============================================================================
-- Migration 216: SPEC-172 tenant scope on original object refs
-- ============================================================================
-- A ref is visible only when its tenant_id matches the session tenant and
-- that tenant owns the session workspace. The delete queue accepts a key
-- only when the key names that same tenant and workspace.

SET search_path = public;

CREATE OR REPLACE FUNCTION public.original_object_refs_enqueue_delete()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    IF position(
        '/t/' || OLD.tenant_id::text || '/w/' || OLD.workspace_id::text || '/'
        IN OLD.object_key
    ) > 0 THEN
        INSERT INTO original_object_deletes (object_key, enqueued_at)
        VALUES (OLD.object_key, now())
        ON CONFLICT (object_key) DO NOTHING;
    END IF;
    RETURN OLD;
END;
$$;

DROP POLICY IF EXISTS tenant_access_guard ON public.original_object_refs;
CREATE POLICY tenant_access_guard ON public.original_object_refs
    AS RESTRICTIVE
    FOR ALL
    TO PUBLIC
    USING (
        tenant_id = public.current_tenant_id()
        AND workspace_id = public.current_workspace_id()
        AND EXISTS (
            SELECT 1 FROM public.workspaces w
            WHERE w.workspace_id = public.current_workspace_id()
              AND w.tenant_id = public.current_tenant_id()
        )
    )
    WITH CHECK (
        tenant_id = public.current_tenant_id()
        AND workspace_id = public.current_workspace_id()
        AND EXISTS (
            SELECT 1 FROM public.workspaces w
            WHERE w.workspace_id = public.current_workspace_id()
              AND w.tenant_id = public.current_tenant_id()
        )
    );
