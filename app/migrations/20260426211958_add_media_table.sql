CREATE TABLE IF NOT EXISTS media (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    airtable_id     TEXT NOT NULL,

    filename        TEXT,
    mime_type       TEXT NOT NULL,
    size_bytes      BIGINT,
    width           INTEGER,
    height          INTEGER,

    url             TEXT NOT NULL,
    thumb_small_url    TEXT,
    thumb_small_width  INTEGER,
    thumb_small_height INTEGER,
    thumb_large_url    TEXT,
    thumb_large_width  INTEGER,
    thumb_large_height INTEGER,
    thumb_full_url     TEXT,
    thumb_full_width   INTEGER,
    thumb_full_height  INTEGER,

    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (project_id, airtable_id)
);

ALTER TABLE projects DROP COLUMN IF EXISTS media_url CASCADE;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS has_media BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_media_project_id ON media(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_media_airtable_id ON media(airtable_id);

CREATE OR REPLACE FUNCTION sync_project_has_media() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE projects SET has_media = TRUE
        WHERE id = NEW.project_id AND NOT has_media;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE projects SET has_media = EXISTS (
            SELECT 1 FROM media WHERE project_id = OLD.project_id
        )
        WHERE id = OLD.project_id;
    ELSIF TG_OP = 'UPDATE' AND OLD.project_id IS DISTINCT FROM NEW.project_id THEN
        UPDATE projects SET has_media = TRUE
        WHERE id = NEW.project_id AND NOT has_media;
        UPDATE projects SET has_media = EXISTS (
            SELECT 1 FROM media WHERE project_id = OLD.project_id
        )
        WHERE id = OLD.project_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER media_sync_has_media
AFTER INSERT OR UPDATE OR DELETE ON media
FOR EACH ROW EXECUTE FUNCTION sync_project_has_media();

CREATE INDEX IF NOT EXISTS idx_media_project_id_id
ON media(project_id, id);
