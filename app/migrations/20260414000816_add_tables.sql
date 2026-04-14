--- add the projects table
CREATE TABLE IF NOT EXISTS projects (
    id SERIAL PRIMARY KEY,
    airtable_id TEXT UNIQUE NOT NULL,
    ysws TEXT NOT NULL,
    approved_at TIMESTAMPTZ,
    code_url TEXT,
    country TEXT,
    demo_url TEXT,
    description TEXT,
    github_username TEXT,
    hours INTEGER,
    screenshot_url TEXT,
    github_stars INTEGER NOT NULL DEFAULT 0,
    display_name TEXT,
    archived_demo TEXT,
    archived_repo TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    embedding vector(1536)
);

CREATE INDEX ON projects USING hnsw (embedding vector_cosine_ops);

--- auto update the updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = NOW();
   RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER set_projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
