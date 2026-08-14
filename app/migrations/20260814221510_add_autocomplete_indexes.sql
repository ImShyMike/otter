-- lets the slack_id project counts run as an index only scan
CREATE INDEX IF NOT EXISTS idx_projects_slack_id_live
ON projects (slack_id)
WHERE deleted_at IS NULL AND slack_id IS NOT NULL;

-- case insensitive prefix lookups
CREATE INDEX IF NOT EXISTS idx_projects_github_username_lower
ON projects (lower(github_username) text_pattern_ops)
WHERE deleted_at IS NULL AND github_username IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_projects_inferred_username_lower
ON projects (lower(inferred_username) text_pattern_ops)
WHERE deleted_at IS NULL AND inferred_username IS NOT NULL;
