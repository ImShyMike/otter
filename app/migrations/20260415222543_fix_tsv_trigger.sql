-- only recompute tsv when its source columns actually change
CREATE OR REPLACE FUNCTION projects_tsv_trigger() RETURNS trigger AS $$
begin
  if TG_OP = 'INSERT'
     or OLD.ysws IS DISTINCT FROM NEW.ysws
     or OLD.description IS DISTINCT FROM NEW.description
     or OLD.country IS DISTINCT FROM NEW.country
     or OLD.display_name IS DISTINCT FROM NEW.display_name
     or OLD.github_username IS DISTINCT FROM NEW.github_username
  then
    new.tsv := to_tsvector('english',
      coalesce(new.ysws, '') || ' ' ||
      left(coalesce(new.description, ''), 1000) || ' ' ||
      coalesce(new.country, '') || ' ' ||
      coalesce(new.display_name, '') || ' ' ||
      coalesce(new.github_username, '')
    );
  end if;
  return new;
end;
$$ LANGUAGE plpgsql;
