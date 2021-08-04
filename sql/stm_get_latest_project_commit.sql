-- returns the timestamp of the latest project commit
CREATE OR REPLACE FUNCTION stm_get_latest_project_commit(_owner_id varchar, _project_id varchar)
RETURNS bigint AS $$ --
DECLARE
  latest_commit bigint;
BEGIN
  select max(commit_ts) into latest_commit from t_commit_ownership where owner_id=_owner_id and project_id=_project_id;
  RETURN latest_commit;
END --
$$ COST 100 STABLE LANGUAGE plpgsql SECURITY DEFINER;

GRANT EXECUTE ON FUNCTION stm_get_latest_project_commit(varchar, varchar) to public;
-- DROP FUNCTION IF EXISTS stm_get_latest_project_commit

/*** TESTING ***/
-- select * from stm_get_latest_project_commit('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK', 'LJq8YVWJt7C9Sxa4poJKea')
-- explain analyze select max(commit_ts) from t_commit_ownership where owner_id='9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK' and project_id='LJq8YVWJt7C9Sxa4poJKea'
