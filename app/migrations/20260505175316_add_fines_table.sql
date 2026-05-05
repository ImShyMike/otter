CREATE TABLE IF NOT EXISTS fines (
  id SERIAL PRIMARY KEY,
  transaction_id VARCHAR(255) NOT NULL,
  amount_cents INTEGER NOT NULL,
  ysws TEXT,
  memo TEXT NOT NULL,
  date DATE NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fines_ysws ON fines(ysws) WHERE ysws IS NOT NULL;

CREATE TRIGGER set_fines_updated_at
    BEFORE UPDATE ON fines
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- deleted generated column for deleted projects
ALTER TABLE project_changes
ADD COLUMN IF NOT EXISTS is_delete boolean
GENERATED ALWAYS AS (
  (changes->'deleted_at'->>'new') IS NOT NULL
  AND (changes->'deleted_at'->>'old') IS NULL
) STORED;

CREATE INDEX IF NOT EXISTS idx_project_changes_is_delete ON project_changes (is_delete) WHERE is_delete = true;
