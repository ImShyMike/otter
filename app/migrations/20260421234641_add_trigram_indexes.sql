-- trigram indexes to accelerate % operator matching used by /search
CREATE INDEX IF NOT EXISTS idx_projects_display_name_trgm
ON projects USING GIN (display_name gin_trgm_ops)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_ysws_trgm
ON projects USING GIN (ysws gin_trgm_ops)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_github_username_trgm
ON projects USING GIN (github_username gin_trgm_ops)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_country_trgm
ON projects USING GIN (country gin_trgm_ops)
WHERE deleted_at IS NULL;
