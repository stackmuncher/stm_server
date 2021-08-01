-- returns all records matching any of the specified commits
-- it is up to the caller to sort out which project id to use
CREATE OR REPLACE FUNCTION stm_find_projects_by_commits(_commit_hash varchar[])
RETURNS SETOF t_commit_ownership AS $$ --
BEGIN --
--
RETURN QUERY
select distinct * from t_commit_ownership where commit_hash = any(_commit_hash);
--
END --
$$ COST 100 STABLE LANGUAGE plpgsql SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_find_projects_by_commits(varchar[]) to public;
-- DROP FUNCTION IF EXISTS stm_find_projects_by_commits

/*** TESTING ***/
explain analyze select * from stm_find_projects_by_commits(array['c1', 'c2'])
explain analyze select * from t_commit_ownership where commit_hash = any(array['c1', 'c2']);
