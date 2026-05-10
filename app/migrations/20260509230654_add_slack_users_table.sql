CREATE TABLE IF NOT EXISTS slack_users (
	slack_id TEXT PRIMARY KEY,
	team_id TEXT,
	name TEXT,
	email TEXT,
	tz TEXT,
	real_name TEXT,
	display_name_normalized TEXT,
	deleted BOOLEAN NOT NULL DEFAULT FALSE,
    image_72 TEXT,
    image_512 TEXT,
	updated_unix BIGINT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_slack_users_email ON slack_users(email) WHERE email IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_slack_users_deleted ON slack_users(deleted) WHERE deleted = FALSE;
