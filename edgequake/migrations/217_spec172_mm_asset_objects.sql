-- ============================================================================
-- Migration 217: SPEC-172 document image and markdown objects
-- ============================================================================
-- Page, figure, chart, and table crops in document_mm_assets, and the
-- extracted markdown in pdf_documents.markdown_content, may live in Postgres
-- (location = database) or in the same private bucket as PDF and non-PDF
-- originals (location = object). The server opens that bucket only when
-- EDGEQUAKE_ORIGINAL_STORE=object.
-- object_id / markdown_object_id are the object owners. asset_path, asset_id,
-- and filenames stay metadata and never enter the key.
-- documents.content and chunk rows stay in Postgres.
-- A CHECK cannot subquery original_object_refs. The same-row checks are the
-- whole rule. There is no down migration (SPEC-150).

SET search_path = public;

ALTER TABLE document_mm_assets
    ADD COLUMN IF NOT EXISTS object_id uuid NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE document_mm_assets
    ADD COLUMN IF NOT EXISTS byte_location text NOT NULL DEFAULT 'database';
ALTER TABLE document_mm_assets
    ALTER COLUMN asset_data DROP NOT NULL;
ALTER TABLE document_mm_assets
    DROP CONSTRAINT IF EXISTS document_mm_assets_byte_location_chk;
ALTER TABLE document_mm_assets
    ADD CONSTRAINT document_mm_assets_byte_location_chk
    CHECK (
        byte_location IN ('database', 'object')
        AND (
            (byte_location = 'database' AND asset_data IS NOT NULL)
            OR (byte_location = 'object' AND asset_data IS NULL)
        )
    );

CREATE UNIQUE INDEX IF NOT EXISTS document_mm_assets_object_id_key
    ON document_mm_assets (object_id);

COMMENT ON COLUMN document_mm_assets.object_id IS
    'SPEC-172 object owner. The bucket key uses this id, never asset_path or asset_id.';
COMMENT ON COLUMN document_mm_assets.byte_location IS
    'database = asset_data present; object = bytes in the originals bucket and asset_data null.';

ALTER TABLE original_object_refs
    ADD COLUMN IF NOT EXISTS asset_object_id uuid NULL
    REFERENCES document_mm_assets(object_id) ON DELETE CASCADE;

ALTER TABLE pdf_documents
    ADD COLUMN IF NOT EXISTS markdown_object_id uuid NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE pdf_documents
    ADD COLUMN IF NOT EXISTS markdown_location text NOT NULL DEFAULT 'database';
ALTER TABLE pdf_documents
    DROP CONSTRAINT IF EXISTS pdf_documents_markdown_location_chk;
ALTER TABLE pdf_documents
    ADD CONSTRAINT pdf_documents_markdown_location_chk
    CHECK (
        markdown_location IN ('database', 'object')
        AND (
            markdown_location = 'database'
            OR markdown_content IS NULL
        )
    );

CREATE UNIQUE INDEX IF NOT EXISTS pdf_documents_markdown_object_id_key
    ON pdf_documents (markdown_object_id);

COMMENT ON COLUMN pdf_documents.markdown_object_id IS
    'SPEC-172 object owner for extracted markdown. The key uses this id, never the filename.';
COMMENT ON COLUMN pdf_documents.markdown_location IS
    'database = markdown_content may be present; object = markdown bytes in the originals bucket and markdown_content null.';

ALTER TABLE original_object_refs
    ADD COLUMN IF NOT EXISTS markdown_object_id uuid NULL
    REFERENCES pdf_documents(markdown_object_id) ON DELETE CASCADE;

ALTER TABLE original_object_refs
    DROP CONSTRAINT IF EXISTS original_object_refs_one_owner;
ALTER TABLE original_object_refs
    ADD CONSTRAINT original_object_refs_one_owner
    CHECK (num_nonnulls(pdf_id, document_id, asset_object_id, markdown_object_id) = 1);

CREATE UNIQUE INDEX IF NOT EXISTS original_object_refs_asset_object_id_key
    ON original_object_refs (asset_object_id) WHERE asset_object_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS original_object_refs_markdown_object_id_key
    ON original_object_refs (markdown_object_id) WHERE markdown_object_id IS NOT NULL;

GRANT SELECT, UPDATE ON document_mm_assets TO edgequake_originals_maint;
GRANT UPDATE ON pdf_documents TO edgequake_originals_maint;
