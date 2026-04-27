ALTER TABLE projects
ADD COLUMN IF NOT EXISTS preview_blurhash TEXT;

ALTER TABLE projects
ADD COLUMN IF NOT EXISTS preview_blurhash_source_key TEXT;
