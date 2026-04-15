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
    embedding vector(1024),
    embedding_model TEXT,
    tsv tsvector
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

--- auto update the tsv column
CREATE OR REPLACE FUNCTION projects_tsv_trigger() RETURNS trigger AS $$
begin
  new.tsv := to_tsvector('english',
    coalesce(new.ysws, '') || ' ' ||
    left(coalesce(new.description, ''), 1000) || ' ' ||
    coalesce(new.country, '') || ' ' ||
    coalesce(new.display_name, '') || ' ' ||
    coalesce(new.github_username, '')
  );
  return new;
end;
$$ LANGUAGE plpgsql;


CREATE TRIGGER tsv_update
    BEFORE INSERT OR UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION projects_tsv_trigger();

--- full text search index
CREATE INDEX idx_projects_tsv ON projects USING GIN (tsv);

-- vector index for similarity search
CREATE INDEX idx_projects_embedding ON projects USING hnsw (embedding vector_cosine_ops);
