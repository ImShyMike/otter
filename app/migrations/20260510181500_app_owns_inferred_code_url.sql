ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS is_github_url BOOLEAN NOT NULL DEFAULT FALSE;

CREATE OR REPLACE FUNCTION projects_tsv_trigger() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT'
         OR OLD.ysws IS DISTINCT FROM NEW.ysws
         OR OLD.description IS DISTINCT FROM NEW.description
         OR OLD.country IS DISTINCT FROM NEW.country
         OR OLD.display_name IS DISTINCT FROM NEW.display_name
         OR OLD.github_username IS DISTINCT FROM NEW.github_username
         OR OLD.inferred_username IS DISTINCT FROM NEW.inferred_username
         OR OLD.inferred_repo IS DISTINCT FROM NEW.inferred_repo
    THEN
        NEW.tsv := to_tsvector('english',
            coalesce(NEW.ysws, '') || ' ' ||
            left(coalesce(NEW.description, ''), 1000) || ' ' ||
            coalesce(NEW.country, '') || ' ' ||
            coalesce(NEW.display_name, '') || ' ' ||
            coalesce(NEW.github_username, '') || ' ' ||
            coalesce(NEW.inferred_username, '') || ' ' ||
            coalesce(replace(replace(NEW.inferred_repo, '-', ' '), '_', ' '), '')
        );
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
