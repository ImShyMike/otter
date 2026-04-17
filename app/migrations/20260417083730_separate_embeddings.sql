-- create separate table for embeddings
CREATE TABLE IF NOT EXISTS project_embeddings (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    embedding vector(1024) NOT NULL,
    model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- migrate existing embeddings
INSERT INTO project_embeddings (project_id, embedding, model)
SELECT id, embedding, embedding_model
FROM projects
WHERE embedding IS NOT NULL AND embedding_model IS NOT NULL;

-- create the HNSW index on the new table
CREATE INDEX idx_project_embeddings_hnsw ON project_embeddings USING hnsw (embedding vector_cosine_ops);

-- drop old columns and redundant indexes
DROP INDEX IF EXISTS projects_embedding_idx;
DROP INDEX IF EXISTS idx_projects_embedding;
ALTER TABLE projects DROP COLUMN IF EXISTS embedding;
ALTER TABLE projects DROP COLUMN IF EXISTS embedding_model;
