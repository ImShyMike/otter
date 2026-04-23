ALTER TABLE projects
ADD COLUMN IF NOT EXISTS inferred_repo TEXT,
ADD COLUMN IF NOT EXISTS inferred_github_username TEXT;

CREATE OR REPLACE FUNCTION projects_tsv_trigger() RETURNS trigger AS $$
DECLARE
	normalized_code_url TEXT;
	owner_raw TEXT;
	repo_raw TEXT;
BEGIN
	normalized_code_url := regexp_replace(
		regexp_replace(
			regexp_replace(coalesce(NEW.code_url, ''), '^git@[^:]+:', ''),
			'^[a-z]+://[^/]+/',
			''
		),
		'\\.git$|/$',
		''
	);

	owner_raw := nullif(split_part(normalized_code_url, '/', 1), '');
	repo_raw := nullif(split_part(normalized_code_url, '/', 2), '');

	NEW.inferred_github_username := CASE
		WHEN length(owner_raw) BETWEEN 1 AND 50 THEN owner_raw
		ELSE NULL
	END;

	NEW.inferred_repo := CASE
		WHEN length(repo_raw) BETWEEN 1 AND 80 THEN repo_raw
		ELSE NULL
	END;

	IF TG_OP = 'INSERT'
		 OR OLD.ysws IS DISTINCT FROM NEW.ysws
		 OR OLD.description IS DISTINCT FROM NEW.description
		 OR OLD.country IS DISTINCT FROM NEW.country
		 OR OLD.display_name IS DISTINCT FROM NEW.display_name
		 OR OLD.github_username IS DISTINCT FROM NEW.github_username
		  OR OLD.inferred_github_username IS DISTINCT FROM NEW.inferred_github_username
		  OR OLD.inferred_repo IS DISTINCT FROM NEW.inferred_repo
		 OR OLD.code_url IS DISTINCT FROM NEW.code_url
	THEN
		NEW.tsv := to_tsvector('english',
			coalesce(NEW.ysws, '') || ' ' ||
			left(coalesce(NEW.description, ''), 1000) || ' ' ||
			coalesce(NEW.country, '') || ' ' ||
			coalesce(NEW.display_name, '') || ' ' ||
			coalesce(NEW.github_username, '') || ' ' ||
			coalesce(NEW.inferred_github_username, '') || ' ' ||
			coalesce(replace(replace(NEW.inferred_repo, '-', ' '), '_', ' '), '')
		);
	END IF;

	RETURN NEW;
END;
$$ LANGUAGE plpgsql;

ALTER TABLE projects DISABLE TRIGGER set_projects_updated_at;

UPDATE projects
SET
	code_url = code_url,
	tsv = tsv;

ALTER TABLE projects ENABLE TRIGGER set_projects_updated_at;

CREATE INDEX IF NOT EXISTS idx_projects_inferred_repo_trgm
ON projects USING GIN (inferred_repo gin_trgm_ops)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_inferred_github_username_trgm
ON projects USING GIN (inferred_github_username gin_trgm_ops)
WHERE deleted_at IS NULL;
