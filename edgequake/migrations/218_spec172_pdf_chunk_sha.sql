-- Packed SHA-256 digests of successive 131072-byte windows of a PDF object.
-- NULL means a partial read is not checked. Adding a nullable column does not
-- rewrite existing rows.

ALTER TABLE public.original_object_refs
    ADD COLUMN IF NOT EXISTS chunk_sha256 bytea;

COMMENT ON COLUMN public.original_object_refs.chunk_sha256 IS
    'Packed SHA-256 digests (32 bytes each) of successive 131072-byte PDF windows. NULL leaves a partial range unchecked.';
