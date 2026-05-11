ALTER TABLE projects ADD COLUMN IF NOT EXISTS slack_id TEXT;

CREATE INDEX IF NOT EXISTS idx_projects_slack_id ON projects(slack_id) WHERE slack_id IS NOT NULL;
