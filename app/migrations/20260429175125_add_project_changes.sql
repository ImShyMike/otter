CREATE TABLE IF NOT EXISTS project_changes (
    id          BIGSERIAL PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    changes     JSONB NOT NULL,
    changed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_project_changes_project_id_changed_at
    ON project_changes (project_id, changed_at DESC);

CREATE INDEX IF NOT EXISTS idx_project_changes_changed_at
    ON project_changes (changed_at DESC);

CREATE OR REPLACE FUNCTION log_project_changes() RETURNS trigger AS $$
DECLARE
    diff JSONB := '{}'::jsonb;
BEGIN
    IF OLD.ysws IS DISTINCT FROM NEW.ysws THEN
        diff := diff || jsonb_build_object('ysws',
            jsonb_build_object('old', to_jsonb(OLD.ysws), 'new', to_jsonb(NEW.ysws)));
    END IF;

    IF OLD.approved_at IS DISTINCT FROM NEW.approved_at THEN
        diff := diff || jsonb_build_object('approved_at',
            jsonb_build_object('old', to_jsonb(OLD.approved_at), 'new', to_jsonb(NEW.approved_at)));
    END IF;

    IF OLD.code_url IS DISTINCT FROM NEW.code_url THEN
        diff := diff || jsonb_build_object('code_url',
            jsonb_build_object('old', to_jsonb(OLD.code_url), 'new', to_jsonb(NEW.code_url)));
    END IF;

    IF OLD.country IS DISTINCT FROM NEW.country THEN
        diff := diff || jsonb_build_object('country',
            jsonb_build_object('old', to_jsonb(OLD.country), 'new', to_jsonb(NEW.country)));
    END IF;

    IF OLD.demo_url IS DISTINCT FROM NEW.demo_url THEN
        diff := diff || jsonb_build_object('demo_url',
            jsonb_build_object('old', to_jsonb(OLD.demo_url), 'new', to_jsonb(NEW.demo_url)));
    END IF;

    IF OLD.description IS DISTINCT FROM NEW.description THEN
        diff := diff || jsonb_build_object('description',
            jsonb_build_object('old', to_jsonb(OLD.description), 'new', to_jsonb(NEW.description)));
    END IF;

    IF OLD.github_username IS DISTINCT FROM NEW.github_username THEN
        diff := diff || jsonb_build_object('github_username',
            jsonb_build_object('old', to_jsonb(OLD.github_username), 'new', to_jsonb(NEW.github_username)));
    END IF;

    IF OLD.hours IS DISTINCT FROM NEW.hours THEN
        diff := diff || jsonb_build_object('hours',
            jsonb_build_object('old', to_jsonb(OLD.hours), 'new', to_jsonb(NEW.hours)));
    END IF;

    IF OLD.true_hours IS DISTINCT FROM NEW.true_hours THEN
        diff := diff || jsonb_build_object('true_hours',
            jsonb_build_object('old', to_jsonb(OLD.true_hours), 'new', to_jsonb(NEW.true_hours)));
    END IF;

    IF OLD.github_stars IS DISTINCT FROM NEW.github_stars THEN
        diff := diff || jsonb_build_object('github_stars',
            jsonb_build_object('old', to_jsonb(OLD.github_stars), 'new', to_jsonb(NEW.github_stars)));
    END IF;

    IF OLD.display_name IS DISTINCT FROM NEW.display_name THEN
        diff := diff || jsonb_build_object('display_name',
            jsonb_build_object('old', to_jsonb(OLD.display_name), 'new', to_jsonb(NEW.display_name)));
    END IF;

    IF OLD.archived_demo IS DISTINCT FROM NEW.archived_demo THEN
        diff := diff || jsonb_build_object('archived_demo',
            jsonb_build_object('old', to_jsonb(OLD.archived_demo), 'new', to_jsonb(NEW.archived_demo)));
    END IF;

    IF OLD.archived_repo IS DISTINCT FROM NEW.archived_repo THEN
        diff := diff || jsonb_build_object('archived_repo',
            jsonb_build_object('old', to_jsonb(OLD.archived_repo), 'new', to_jsonb(NEW.archived_repo)));
    END IF;

    IF OLD.deleted_at IS DISTINCT FROM NEW.deleted_at THEN
        diff := diff || jsonb_build_object('deleted_at',
            jsonb_build_object('old', to_jsonb(OLD.deleted_at), 'new', to_jsonb(NEW.deleted_at)));
    END IF;

    IF diff <> '{}'::jsonb THEN
        INSERT INTO project_changes (project_id, changes)
        VALUES (NEW.id, diff);
    END IF;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER projects_log_changes
    AFTER UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION log_project_changes();
